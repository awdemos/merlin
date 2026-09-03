use crate::config::TargetConfig;
use crate::protocol::Request;
use crate::routing::{AlgorithmOutcome, RoutingAlgorithm};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct UpperConfidenceBound {
    targets: Vec<TargetConfig>,
    c: f64,
    counts: Mutex<HashMap<String, u64>>,
    rewards: Mutex<HashMap<String, f64>>,
}

impl UpperConfidenceBound {
    pub fn new(targets: Vec<TargetConfig>, c: f64) -> Self {
        Self {
            targets,
            c,
            counts: Mutex::new(HashMap::new()),
            rewards: Mutex::new(HashMap::new()),
        }
    }
}

impl RoutingAlgorithm for UpperConfidenceBound {
    fn select(
        &self,
        _request: &Request,
        _targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome> {
        let counts = self.counts.lock().unwrap();
        let rewards = self.rewards.lock().unwrap();
        let total = counts.values().sum::<u64>() as f64;
        let mut best: Option<(String, f64)> = None;
        for t in &self.targets {
            let n = *counts.get(&t.name).unwrap_or(&0) as f64;
            let r = *rewards.get(&t.name).unwrap_or(&0.0);
            let avg = if n > 0.0 { r / n } else { 0.0 };
            let bonus = if total > 0.0 && n > 0.0 {
                self.c * (total.ln() / n).sqrt()
            } else {
                f64::INFINITY
            };
            let score = avg + bonus;
            let is_better = if let Some((_, s)) = best {
                score > s
            } else {
                true
            };
            if is_better {
                best = Some((t.name.clone(), score));
            }
        }
        let selected = best.map(|(n, _)| n).unwrap_or_else(|| {
            self.targets
                .first()
                .map(|t| t.name.clone())
                .unwrap_or_default()
        });
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

    fn record_reward(&self, target: &str, _request: &Request, reward: f64) {
        *self
            .counts
            .lock()
            .unwrap()
            .entry(target.to_string())
            .or_insert(0) += 1;
        *self
            .rewards
            .lock()
            .unwrap()
            .entry(target.to_string())
            .or_insert(0.0) += reward;
    }
}
