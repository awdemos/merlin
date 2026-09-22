//! LinUCB-style contextual bandit routing.
//!
//! Each target ("arm") owns a disjoint 10-dim linear ridge model over the
//! context vector produced by [`PromptFeatures::vector`] (bias, difficulty,
//! needs_tools, is_sensitive, 6-dim domain one-hot). `A` starts at identity,
//! `b` at zero; selection scores `p = theta·x + alpha * sqrt(xᵀ A⁻¹ x)` with
//! `theta = A⁻¹b`, picking the argmax (ties resolve to the first target in
//! configured order). Rewards update `A += x·xᵀ`, `b += reward·x`.
//!
//! Cold start (zero total pulls) picks arms by a stable hash of the prompt
//! (sum of bytes) so the first distinct prompts spread across targets before
//! learning kicks in. When sidecar features are unavailable, a degraded
//! [`PromptFeatures::legacy`] vector (difficulty estimated from prompt
//! length) is used instead, preserving a useful learning signal.
//!
//! This replaces the previous byte-hash placeholder, which ignored rewards.

use crate::config::TargetConfig;
use crate::features::PromptFeatures;
use crate::protocol::Request;
use crate::routing::{AlgorithmOutcome, RoutingAlgorithm};
use std::collections::HashMap;
use std::sync::Mutex;

/// Dimension of the context vector (4 scalar features + 6 domain one-hot).
pub const CONTEXT_DIM: usize = 10;

#[derive(Debug, Clone)]
struct ArmState {
    a: [[f64; CONTEXT_DIM]; CONTEXT_DIM],
    b: [f64; CONTEXT_DIM],
    pulls: u64,
}

impl ArmState {
    fn new() -> Self {
        let mut a = [[0.0f64; CONTEXT_DIM]; CONTEXT_DIM];
        for (i, row) in a.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        Self {
            a,
            b: [0.0; CONTEXT_DIM],
            pulls: 0,
        }
    }
}

/// Disjoint per-arm LinUCB over prompt-feature context vectors.
#[derive(Debug)]
pub struct ContextualBandit {
    targets: Vec<TargetConfig>,
    /// LinUCB exploration coefficient; taken from `exploration_rate`.
    alpha: f64,
    #[allow(dead_code)]
    learning_rate: f64,
    arms: Mutex<HashMap<String, ArmState>>,
}

impl ContextualBandit {
    pub fn new(
        targets: Vec<TargetConfig>,
        learning_rate: f64,
        exploration_rate: f64,
    ) -> anyhow::Result<Self> {
        // exploration_rate doubles as the LinUCB alpha: larger values push
        // selection toward under-explored arms for a given context.
        Ok(Self {
            targets,
            alpha: exploration_rate,
            learning_rate,
            arms: Mutex::new(HashMap::new()),
        })
    }

    fn context_vector(&self, request: &Request) -> [f64; CONTEXT_DIM] {
        let v = request
            .features
            .clone()
            .unwrap_or_else(|| PromptFeatures::legacy(request.llm_request.prompt_text().len()))
            .vector();
        v.try_into()
            .expect("PromptFeatures::vector always yields CONTEXT_DIM dims")
    }
}

impl RoutingAlgorithm for ContextualBandit {
    fn needs_features(&self) -> bool {
        true
    }

    fn select(
        &self,
        request: &Request,
        _targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome> {
        let prompt = request.llm_request.prompt_text();
        let arms = self.arms.lock().unwrap();
        let total_pulls: u64 = arms.values().map(|a| a.pulls).sum();

        let selected = if total_pulls == 0 && !self.targets.is_empty() {
            // Cold start: stable hash spreads the first distinct prompts
            // across targets (same behavior as the old hash placeholder,
            // but only until the first reward is recorded).
            let idx = if prompt.is_empty() {
                0
            } else {
                prompt.bytes().map(|b| b as usize).sum::<usize>() % self.targets.len()
            };
            self.targets[idx].name.clone()
        } else {
            let x = self.context_vector(request);
            let mut best: Option<(String, f64)> = None;
            for t in &self.targets {
                let arm = arms.get(&t.name).cloned().unwrap_or_else(ArmState::new);
                let p = arm_score(&arm, &x, self.alpha);
                let is_better = match &best {
                    Some((_, score)) => p > *score,
                    None => true,
                };
                if is_better {
                    best = Some((t.name.clone(), p));
                }
            }
            best.map(|(name, _)| name).unwrap_or_default()
        };
        let fallbacks = self
            .targets
            .iter()
            .filter(|t| t.name != selected)
            .map(|t| t.name.clone())
            .collect();
        Ok(AlgorithmOutcome {
            selected,
            fallbacks,
        })
    }

    fn record_reward(&self, target: &str, request: &Request, reward: f64) {
        let x = self.context_vector(request);
        let mut arms = self.arms.lock().unwrap();
        let arm = arms.entry(target.to_string()).or_insert_with(ArmState::new);
        for i in 0..CONTEXT_DIM {
            for j in 0..CONTEXT_DIM {
                arm.a[i][j] += x[i] * x[j];
            }
            arm.b[i] += reward * x[i];
        }
        arm.pulls += 1;
    }
}

/// LinUCB arm score: `theta·x + alpha * sqrt(xᵀ A⁻¹ x)`, `theta = A⁻¹b`.
/// A singular `A` (practically impossible with identity initialization)
/// yields negative infinity so the arm is never chosen.
fn arm_score(arm: &ArmState, x: &[f64; CONTEXT_DIM], alpha: f64) -> f64 {
    let theta = match solve(&arm.a, &arm.b) {
        Some(theta) => theta,
        None => return f64::NEG_INFINITY,
    };
    let mean: f64 = theta.iter().zip(x.iter()).map(|(t, xi)| t * xi).sum();
    if alpha == 0.0 {
        return mean;
    }
    // xᵀ A⁻¹ x, computed by solving A·y = x once.
    let y = match solve(&arm.a, x) {
        Some(y) => y,
        None => return f64::NEG_INFINITY,
    };
    let variance: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| xi * yi).sum();
    mean + alpha * variance.max(0.0).sqrt()
}

/// Solve `A·x = b` for a 10x10 system via Gaussian elimination with partial
/// pivoting. Returns `None` if the matrix is singular.
fn solve(
    a: &[[f64; CONTEXT_DIM]; CONTEXT_DIM],
    b: &[f64; CONTEXT_DIM],
) -> Option<[f64; CONTEXT_DIM]> {
    const N: usize = CONTEXT_DIM;
    const EPS: f64 = 1e-12;

    let mut aug = [[0.0f64; N + 1]; N];
    for i in 0..N {
        aug[i][..N].copy_from_slice(&a[i]);
        aug[i][N] = b[i];
    }

    for col in 0..N {
        let mut pivot = col;
        for row in col + 1..N {
            if aug[row][col].abs() > aug[pivot][col].abs() {
                pivot = row;
            }
        }
        if aug[pivot][col].abs() < EPS {
            return None;
        }
        aug.swap(col, pivot);
        let pivot_val = aug[col][col];
        for row in col + 1..N {
            let factor = aug[row][col] / pivot_val;
            let (above, below) = aug.split_at_mut(row);
            for (cell, pivot_cell) in below[0][col..=N].iter_mut().zip(&above[col][col..=N]) {
                *cell -= factor * pivot_cell;
            }
        }
    }

    let mut x = [0.0f64; N];
    for row in (0..N).rev() {
        let mut sum = aug[row][N];
        for c in row + 1..N {
            sum -= aug[row][c] * x[c];
        }
        x[row] = sum / aug[row][row];
    }
    Some(x)
}
