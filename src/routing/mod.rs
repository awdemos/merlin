//! Routing policies for provider selection.
//!
//! This module implements various bandit algorithms for intelligent provider selection:
//!
//! - **Epsilon-Greedy**: Simple exploration/exploitation tradeoff
//! - **Thompson Sampling**: Bayesian approach with Beta distributions
//! - **Upper Confidence Bound (UCB)**: Optimistic exploration bonus
//! - **Contextual Bandit**: Feature-based linear model

use rand::Rng;
use rand_distr::{Beta, Distribution};
use std::collections::HashMap;

/// Minimum prediction value for contextual bandit predictions.
/// Values below this are clamped to prevent extreme negative predictions that could
/// cause numerical instability or unreasonable routing decisions.
const CONTEXTUAL_PREDICTION_MIN: f64 = -1000.0;

/// Maximum prediction value for contextual bandit predictions.
/// Values above this are clamped to prevent extreme positive predictions that could
/// cause numerical instability or unreasonable routing decisions.
/// The range [-1000.0, 1000.0] provides a wide enough range for normalized reward signals
/// while preventing infinity or NaN propagation from numerical errors.
const CONTEXTUAL_PREDICTION_MAX: f64 = 1000.0;

/// Epsilon threshold for detecting near-zero feature norm values.
/// Used to prevent division by zero in prediction calculations.
/// This is a standard machine learning numerical stability constant.
const FEATURE_NORM_EPSILON: f64 = 1e-10;

/// Routing policy for selecting among multiple LLM providers.
///
/// Each variant implements a different strategy for balancing exploration
/// (trying new providers) and exploitation (using known good providers).
pub enum RoutingPolicy {
    /// Epsilon-greedy policy: explore with probability epsilon, otherwise exploit.
    EpsilonGreedy {
        /// Probability of random exploration (0.0 to 1.0).
        epsilon: f64,
    },
    /// Thompson sampling using Beta distributions for each provider.
    ThompsonSampling {
        /// Per-provider arm statistics.
        arms: HashMap<usize, ThompsonArm>,
    },
    /// Upper Confidence Bound policy with optimistic exploration.
    UpperConfidenceBound {
        /// Per-provider arm statistics.
        arms: HashMap<usize, UCBArm>,
        /// Confidence level for exploration bonus calculation.
        confidence_level: f64,
        /// Total number of rounds played.
        total_rounds: u32,
    },
    /// Contextual bandit using linear models for feature-based selection.
    Contextual {
        /// Per-provider contextual arms with learned weights.
        arms: HashMap<usize, ContextualArm>,
        /// Dimension of feature vectors.
        feature_dim: usize,
        /// Learning rate for weight updates.
        learning_rate: f64,
        /// Probability of random exploration.
        exploration_rate: f64,
    },
}

/// Thompson sampling arm with Beta distribution parameters.
#[derive(Clone, Debug)]
pub struct ThompsonArm {
    /// Alpha parameter (successes + 1 for uniform prior).
    pub alpha: f64,
    /// Beta parameter (failures + 1 for uniform prior).
    pub beta: f64,
}

/// Upper Confidence Bound arm with running statistics.
#[derive(Clone, Debug)]
pub struct UCBArm {
    /// Cumulative reward received.
    pub total_reward: f64,
    /// Number of times this arm has been pulled.
    pub num_pulls: u32,
    /// Average reward (total_reward / num_pulls).
    pub average_reward: f64,
}

/// Contextual bandit arm with linear model weights.
#[derive(Clone, Debug)]
pub struct ContextualArm {
    /// Feature weights for the linear model.
    pub weights: Vec<f64>,
    /// Cumulative reward received.
    pub total_reward: f64,
    /// Number of times this arm has been pulled.
    pub num_pulls: u32,
    /// L2 norm of weights for normalization.
    pub feature_norm: f64,
}

impl ThompsonArm {
    /// Creates a new Thompson arm with uniform prior (alpha=1, beta=1).
    pub fn new() -> Self {
        ThompsonArm {
            alpha: 1.0, // Uniform prior
            beta: 1.0,
        }
    }

    pub fn sample(&self) -> f64 {
        match Beta::new(self.alpha, self.beta) {
            Ok(beta_dist) => beta_dist.sample(&mut rand::thread_rng()),
            Err(_) => {
                tracing::warn!(
                    "Invalid Beta distribution parameters: alpha={}, beta={}. Using fallback.",
                    self.alpha,
                    self.beta
                );
                0.5
            }
        }
    }

    /// Records a successful outcome, incrementing alpha.
    pub fn update_success(&mut self) {
        self.alpha += 1.0;
    }

    /// Records a failed outcome, incrementing beta.
    pub fn update_failure(&mut self) {
        self.beta += 1.0;
    }
}

impl UCBArm {
    /// Creates a new UCB arm with zero initial statistics.
    pub fn new() -> Self {
        UCBArm {
            total_reward: 0.0,
            num_pulls: 0,
            average_reward: 0.0,
        }
    }

    /// Updates the arm with a new reward value.
    pub fn update_reward(&mut self, reward: f64) {
        self.num_pulls += 1;
        self.total_reward += reward;
        self.average_reward = self.total_reward / self.num_pulls as f64;
    }

    /// Calculates the UCB value for this arm.
    ///
    /// Returns infinity if the arm hasn't been pulled yet to encourage exploration.
    pub fn calculate_ucb_value(&self, total_rounds: u32, confidence_level: f64) -> f64 {
        if self.num_pulls == 0 {
            return f64::INFINITY;
        }

        let exploration_bonus =
            confidence_level * ((total_rounds as f64).ln() / self.num_pulls as f64).sqrt();

        self.average_reward + exploration_bonus
    }
}

impl ContextualArm {
    /// Creates a new contextual arm with zero-initialized weights.
    pub fn new(feature_dim: usize) -> Self {
        ContextualArm {
            weights: vec![0.0; feature_dim],
            total_reward: 0.0,
            num_pulls: 0,
            feature_norm: 1.0,
        }
    }

    /// Predicts the expected reward for the given feature vector.
    pub fn predict(&self, features: &[f64]) -> f64 {
        if features.len() != self.weights.len() {
            return 0.0;
        }

        // Dot product of weights and features
        let mut prediction = 0.0;
        for (w, f) in self.weights.iter().zip(features.iter()) {
            prediction += w * f;
        }

        // Safe division with fallback
        if self.feature_norm.abs() < FEATURE_NORM_EPSILON {
            return 0.0;
        }

        let result = prediction / self.feature_norm;

        // Clamp to reasonable range to prevent NaN/Inf
        result.clamp(CONTEXTUAL_PREDICTION_MIN, CONTEXTUAL_PREDICTION_MAX)
    }

    /// Updates weights using online gradient descent.
    pub fn update(&mut self, features: &[f64], reward: f64, learning_rate: f64) {
        if features.len() != self.weights.len() {
            return;
        }

        self.num_pulls += 1;
        self.total_reward += reward;

        // Online gradient descent update
        let prediction = self.predict(features);
        let error = reward - prediction;

        // Update weights based on gradient
        for (i, feature) in features.iter().enumerate() {
            self.weights[i] += learning_rate * error * feature;
        }

        // Update feature norm for normalization
        self.feature_norm = self.weights.iter().map(|w| w * w).sum::<f64>().sqrt();
        if self.feature_norm == 0.0 {
            self.feature_norm = 1.0;
        }
    }
}

impl RoutingPolicy {
    /// Creates a new Thompson Sampling policy with the specified number of providers.
    pub fn new_thompson_sampling(num_providers: usize) -> Self {
        let mut arms = HashMap::new();
        for i in 0..num_providers {
            arms.insert(i, ThompsonArm::new());
        }
        RoutingPolicy::ThompsonSampling { arms }
    }

    /// Creates a new UCB policy with the specified confidence level.
    pub fn new_upper_confidence_bound(num_providers: usize, confidence_level: f64) -> Self {
        let mut arms = HashMap::new();
        for i in 0..num_providers {
            arms.insert(i, UCBArm::new());
        }
        RoutingPolicy::UpperConfidenceBound {
            arms,
            confidence_level,
            total_rounds: 0,
        }
    }

    /// Creates a new contextual bandit policy.
    pub fn new_contextual(
        num_providers: usize,
        feature_dim: usize,
        learning_rate: f64,
        exploration_rate: f64,
    ) -> Self {
        let mut arms = HashMap::new();
        for i in 0..num_providers {
            arms.insert(i, ContextualArm::new(feature_dim));
        }
        RoutingPolicy::Contextual {
            arms,
            feature_dim,
            learning_rate,
            exploration_rate,
        }
    }

    /// Selects a provider index without context features.
    pub fn select_index(&self, num_providers: usize) -> usize {
        self.select_index_with_context(num_providers, &[])
    }

    /// Selects a provider index using context features for contextual policies.
    pub fn select_index_with_context(
        &self,
        num_providers: usize,
        context_features: &[f64],
    ) -> usize {
        match self {
            RoutingPolicy::EpsilonGreedy { epsilon } => {
                if rand::thread_rng().gen_bool(*epsilon) {
                    rand::thread_rng().gen_range(0..num_providers)
                } else {
                    0 // Default to first provider for now
                }
            }
            RoutingPolicy::ThompsonSampling { arms } => {
                let mut best_index = 0;
                let mut best_sample = 0.0;
                let default_arm = ThompsonArm::new();

                for i in 0..num_providers {
                    let arm = arms.get(&i).unwrap_or(&default_arm);
                    let sample = arm.sample();
                    if sample > best_sample {
                        best_sample = sample;
                        best_index = i;
                    }
                }
                best_index
            }
            RoutingPolicy::UpperConfidenceBound {
                arms,
                confidence_level,
                total_rounds,
            } => {
                let mut best_index = 0;
                let mut best_ucb_value = f64::NEG_INFINITY;
                let default_arm = UCBArm::new();

                for i in 0..num_providers {
                    let arm = arms.get(&i).unwrap_or(&default_arm);
                    let ucb_value = arm.calculate_ucb_value(*total_rounds, *confidence_level);
                    if ucb_value > best_ucb_value {
                        best_ucb_value = ucb_value;
                        best_index = i;
                    }
                }
                best_index
            }
            RoutingPolicy::Contextual {
                arms,
                exploration_rate,
                ..
            } => {
                let mut best_index = 0;
                let mut best_score = f64::NEG_INFINITY;
                let default_arm = ContextualArm::new(context_features.len());

                // Explore or exploit
                if rand::thread_rng().gen_bool(*exploration_rate) {
                    // Exploration: random selection
                    best_index = rand::thread_rng().gen_range(0..num_providers);
                } else {
                    // Exploitation: use learned model
                    for i in 0..num_providers {
                        let arm = arms.get(&i).unwrap_or(&default_arm);
                        let score = arm.predict(context_features);
                        if score > best_score {
                            best_score = score;
                            best_index = i;
                        }
                    }
                }
                best_index
            }
        }
    }

    /// Updates the policy with a binary success/failure outcome.
    pub fn update_reward(&mut self, provider_index: usize, success: bool) {
        match self {
            RoutingPolicy::ThompsonSampling { arms } => {
                if let Some(arm) = arms.get_mut(&provider_index) {
                    if success {
                        arm.update_success();
                    } else {
                        arm.update_failure();
                    }
                }
            }
            RoutingPolicy::UpperConfidenceBound {
                arms, total_rounds, ..
            } => {
                *total_rounds += 1;
                if let Some(arm) = arms.get_mut(&provider_index) {
                    let reward = if success { 1.0 } else { 0.0 };
                    arm.update_reward(reward);
                }
            }
            RoutingPolicy::EpsilonGreedy { .. } => {
                // EpsilonGreedy doesn't learn from rewards in this simple implementation
            }
            RoutingPolicy::Contextual { .. } => {
                // Contextual bandit requires context features for updates
                // Use update_reward_with_context instead
            }
        }
    }

    /// Updates the policy with a continuous reward score (0.0 to 1.0).
    pub fn update_reward_with_score(&mut self, provider_index: usize, reward_score: f64) {
        match self {
            RoutingPolicy::ThompsonSampling { arms } => {
                if let Some(arm) = arms.get_mut(&provider_index) {
                    if reward_score > 0.5 {
                        arm.update_success();
                    } else {
                        arm.update_failure();
                    }
                }
            }
            RoutingPolicy::UpperConfidenceBound {
                arms, total_rounds, ..
            } => {
                *total_rounds += 1;
                if let Some(arm) = arms.get_mut(&provider_index) {
                    arm.update_reward(reward_score);
                }
            }
            RoutingPolicy::EpsilonGreedy { .. } => {
                // EpsilonGreedy doesn't learn from rewards in this simple implementation
            }
            RoutingPolicy::Contextual { .. } => {
                // Contextual bandit requires context features for updates
                // Use update_reward_with_context instead
            }
        }
    }

    /// Updates a contextual policy with features and reward score.
    ///
    /// For non-contextual policies, falls back to `update_reward_with_score`.
    pub fn update_reward_with_context(
        &mut self,
        provider_index: usize,
        context_features: &[f64],
        reward_score: f64,
    ) {
        match self {
            RoutingPolicy::Contextual {
                arms,
                learning_rate,
                ..
            } => {
                if let Some(arm) = arms.get_mut(&provider_index) {
                    arm.update(context_features, reward_score, *learning_rate);
                }
            }
            _ => {
                // For non-contextual policies, fall back to simple update
                self.update_reward_with_score(provider_index, reward_score);
            }
        }
    }
}

#[cfg(test)]
mod contextual_tests;
