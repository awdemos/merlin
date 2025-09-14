use rand::Rng;
use rand_distr::{Beta, Distribution};
use std::collections::HashMap;

pub enum RoutingPolicy {
    EpsilonGreedy { epsilon: f64 },
    ThompsonSampling {
        arms: HashMap<usize, ThompsonArm>,
    },
    UpperConfidenceBound {
        arms: HashMap<usize, UCBArm>,
        confidence_level: f64,
        total_rounds: u32,
    },
    Contextual {
        arms: HashMap<usize, ContextualArm>,
        feature_dim: usize,
        learning_rate: f64,
        exploration_rate: f64,
    },
}

#[derive(Clone, Debug)]
pub struct ThompsonArm {
    pub alpha: f64, // Successes + 1 (prior)
    pub beta: f64,  // Failures + 1 (prior)
}

#[derive(Clone, Debug)]
pub struct UCBArm {
    pub total_reward: f64,
    pub num_pulls: u32,
    pub average_reward: f64,
}

#[derive(Clone, Debug)]
pub struct ContextualArm {
    pub weights: Vec<f64>, // Feature weights for linear model
    pub total_reward: f64,
    pub num_pulls: u32,
    pub feature_norm: f64, // For normalization
}

impl ThompsonArm {
    pub fn new() -> Self {
        ThompsonArm {
            alpha: 1.0, // Uniform prior
            beta: 1.0,
        }
    }

    pub fn sample(&self) -> f64 {
        let beta_dist = Beta::new(self.alpha, self.beta).unwrap();
        beta_dist.sample(&mut rand::thread_rng())
    }

    pub fn update_success(&mut self) {
        self.alpha += 1.0;
    }

    pub fn update_failure(&mut self) {
        self.beta += 1.0;
    }
}

impl UCBArm {
    pub fn new() -> Self {
        UCBArm {
            total_reward: 0.0,
            num_pulls: 0,
            average_reward: 0.0,
        }
    }

    pub fn update_reward(&mut self, reward: f64) {
        self.num_pulls += 1;
        self.total_reward += reward;
        self.average_reward = self.total_reward / self.num_pulls as f64;
    }

    pub fn calculate_ucb_value(&self, total_rounds: u32, confidence_level: f64) -> f64 {
        if self.num_pulls == 0 {
            return f64::INFINITY; // Explore arms that haven't been pulled
        }

        let exploration_bonus = confidence_level *
            ((total_rounds as f64).ln() / self.num_pulls as f64).sqrt();

        self.average_reward + exploration_bonus
    }
}

impl ContextualArm {
    pub fn new(feature_dim: usize) -> Self {
        ContextualArm {
            weights: vec![0.0; feature_dim], // Initialize with small random values
            total_reward: 0.0,
            num_pulls: 0,
            feature_norm: 1.0,
        }
    }

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
        if self.feature_norm.abs() < 1e-10 {
            return 0.0;
        }

        let result = prediction / self.feature_norm;

        // Clamp to reasonable range to prevent NaN/Inf
        result.clamp(-1000.0, 1000.0)
    }

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
    pub fn new_thompson_sampling(num_providers: usize) -> Self {
        let mut arms = HashMap::new();
        for i in 0..num_providers {
            arms.insert(i, ThompsonArm::new());
        }
        RoutingPolicy::ThompsonSampling { arms }
    }

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

    pub fn new_contextual(num_providers: usize, feature_dim: usize, learning_rate: f64, exploration_rate: f64) -> Self {
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

    pub fn select_index(&self, num_providers: usize) -> usize {
        self.select_index_with_context(num_providers, &[])
    }

    pub fn select_index_with_context(&self, num_providers: usize, context_features: &[f64]) -> usize {
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
            RoutingPolicy::UpperConfidenceBound { arms, confidence_level, total_rounds } => {
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
            RoutingPolicy::Contextual { arms, exploration_rate, .. } => {
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
            RoutingPolicy::UpperConfidenceBound { arms, total_rounds, .. } => {
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
            RoutingPolicy::UpperConfidenceBound { arms, total_rounds, .. } => {
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

    pub fn update_reward_with_context(&mut self, provider_index: usize, context_features: &[f64], reward_score: f64) {
        match self {
            RoutingPolicy::Contextual { arms, learning_rate, .. } => {
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
