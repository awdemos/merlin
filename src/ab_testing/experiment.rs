// src/ab_testing/experiment.rs
use crate::ab_testing::{ExperimentConfig, VariantConfig, ExperimentMetrics, ExperimentStorage};
use crate::routing::RoutingPolicy;
use std::collections::HashMap;
use rand::{thread_rng, Rng};
use anyhow::Result;
use serde::{Serialize, Deserialize};

pub struct Experiment {
    pub config: ExperimentConfig,
    pub variants: HashMap<String, Variant>,
    pub user_assignments: HashMap<String, String>, // user_id -> variant_id
}

pub struct Variant {
    pub config: VariantConfig,
    pub routing_policy: RoutingPolicy,
    pub metrics: ExperimentMetrics,
}

impl Experiment {
    pub fn new(config: ExperimentConfig) -> Result<Self> {
        config.validate().map_err(|e| anyhow::anyhow!(e))?;

        let mut variants = HashMap::new();
        for variant_config in &config.variants {
            let routing_policy = Self::create_routing_policy(&variant_config.routing_policy)?;
            let metrics = ExperimentMetrics::new(variant_config.id.clone());

            variants.insert(
                variant_config.id.clone(),
                Variant {
                    config: variant_config.clone(),
                    routing_policy,
                    metrics,
                }
            );
        }

        Ok(Self {
            config,
            variants,
            user_assignments: HashMap::new(),
        })
    }

    pub fn assign_user(&mut self, user_id: &str) -> Option<&Variant> {
        // Check if user is already assigned
        if let Some(variant_id) = self.user_assignments.get(user_id) {
            return self.variants.get(variant_id);
        }

        // Check if experiment is running and user is eligible
        if !self.config.can_run() {
            return None;
        }

        // Check traffic allocation
        if !self.is_user_in_experiment(user_id) {
            return None;
        }

        // Assign to a variant based on weights
        let variant_id = self.select_variant_for_user(user_id);
        self.user_assignments.insert(user_id.to_string(), variant_id.clone());
        self.variants.get(&variant_id)
    }

    pub fn record_interaction(&mut self, _user_id: &str, variant_id: &str, metrics: &InteractionMetrics) {
        if let Some(variant) = self.variants.get_mut(variant_id) {
            variant.metrics.record_interaction(metrics);
        }
    }

    pub fn get_results(&self) -> ExperimentResults {
        let mut variant_results = HashMap::new();

        for (variant_id, variant) in &self.variants {
            variant_results.insert(variant_id.clone(), VariantResult {
                config: variant.config.clone(),
                metrics: variant.metrics.clone(),
                sample_size: variant.metrics.total_interactions,
                confidence_interval: self.calculate_confidence_interval(variant),
                is_winner: self.is_variant_winner(variant),
            });
        }

        ExperimentResults {
            experiment_id: self.config.id.clone(),
            experiment_name: self.config.name.clone(),
            status: self.config.status.clone(),
            variant_results,
            total_users: self.user_assignments.len() as u32,
            statistical_significance: self.calculate_statistical_significance(),
            recommendation: self.generate_recommendation(),
        }
    }

    fn is_user_in_experiment(&self, user_id: &str) -> bool {
        // Use consistent hashing for user assignment
        let hash = self.hash_user_id(user_id);
        hash < self.config.traffic_allocation
    }

    fn hash_user_id(&self, user_id: &str) -> f64 {
        // Simple hash function - in production, use a proper hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        let hash = hasher.finish();
        (hash as f64) / (u64::MAX as f64)
    }

    fn select_variant_for_user(&self, _user_id: &str) -> String {
        let mut rng = thread_rng();
        let rand_val: f64 = rng.gen();

        let mut cumulative_weight = 0.0;
        for variant in &self.config.variants {
            cumulative_weight += variant.weight;
            if rand_val <= cumulative_weight {
                return variant.id.clone();
            }
        }

        // Fallback to last variant
        self.config.variants.last().unwrap().id.clone()
    }

    fn create_routing_policy(config: &crate::ab_testing::config::RoutingPolicyConfig) -> Result<RoutingPolicy> {
        use crate::ab_testing::config::RoutingPolicyConfig::*;

        match config {
            EpsilonGreedy { epsilon } => {
                Ok(RoutingPolicy::EpsilonGreedy { epsilon: *epsilon })
            },
            ThompsonSampling { arms_count } => {
                let mut arms = HashMap::new();
                for i in 0..*arms_count {
                    arms.insert(i, crate::routing::ThompsonArm::new());
                }
                Ok(RoutingPolicy::ThompsonSampling { arms })
            },
            UpperConfidenceBound { confidence_level } => {
                let arms_count = 3; // Default, should be configurable
                let mut arms = HashMap::new();
                for i in 0..arms_count {
                    arms.insert(i, crate::routing::UCBArm::new());
                }
                Ok(RoutingPolicy::UpperConfidenceBound {
                    arms,
                    confidence_level: *confidence_level,
                    total_rounds: 0,
                })
            },
            Contextual { feature_dim, learning_rate } => {
                let arms_count = 3; // Default, should be configurable
                let mut arms = HashMap::new();
                for i in 0..arms_count {
                    arms.insert(i, crate::routing::ContextualArm::new(*feature_dim));
                }
                Ok(RoutingPolicy::Contextual {
                    arms,
                    feature_dim: *feature_dim,
                    learning_rate: *learning_rate,
                    exploration_rate: 0.1, // Default
                })
            },
            Static { provider_index: _ } => {
                // For static routing, we'll use a simple epsilon-greedy with very low epsilon
                Ok(RoutingPolicy::EpsilonGreedy { epsilon: 0.01 })
            }
        }
    }

    fn calculate_confidence_interval(&self, variant: &Variant) -> (f64, f64) {
        // Simple 95% confidence interval calculation
        let mean = variant.metrics.average_success_rate();
        let std_dev = variant.metrics.success_rate_std_dev();
        let sample_size = variant.metrics.total_interactions.max(1) as f64;

        let margin_of_error = 1.96 * std_dev / sample_size.sqrt();
        (mean - margin_of_error, mean + margin_of_error)
    }

    fn is_variant_winner(&self, variant: &Variant) -> bool {
        if variant.config.is_control {
            return false;
        }

        // Compare with control variant
        if let Some(control_variant) = self.variants.values().find(|v| v.config.is_control) {
            variant.metrics.average_success_rate() > control_variant.metrics.average_success_rate()
        } else {
            false
        }
    }

    fn calculate_statistical_significance(&self) -> Option<f64> {
        // Simple t-test implementation
        // In production, use a proper statistical library
        let variants: Vec<_> = self.variants.values().collect();
        if variants.len() < 2 {
            return None;
        }

        let control = variants.iter().find(|v| v.config.is_control)?;
        let treatment = variants.iter().find(|v| !v.config.is_control)?;

        let control_mean = control.metrics.average_success_rate();
        let treatment_mean = treatment.metrics.average_success_rate();
        let control_std = control.metrics.success_rate_std_dev();
        let treatment_std = treatment.metrics.success_rate_std_dev();
        let control_n = control.metrics.total_interactions.max(1) as f64;
        let treatment_n = treatment.metrics.total_interactions.max(1) as f64;

        // Pooled standard deviation
        let pooled_std = ((control_n - 1.0) * control_std.powi(2) + (treatment_n - 1.0) * treatment_std.powi(2))
            / (control_n + treatment_n - 2.0);
        let pooled_std = pooled_std.sqrt();

        // T-statistic
        let t_stat = (treatment_mean - control_mean) / (pooled_std * (1.0 / control_n + 1.0 / treatment_n).sqrt());

        // Degrees of freedom
        let df = control_n + treatment_n - 2.0;

        // For simplicity, return |t_stat| / df as a pseudo p-value
        // In production, use proper t-distribution calculation
        Some((t_stat.abs() / df).min(1.0))
    }

    fn generate_recommendation(&self) -> ExperimentRecommendation {
        let results = self.get_results();

        if results.total_users < self.config.success_criteria.min_sample_size {
            return ExperimentRecommendation::Continue;
        }

        if let Some(p_value) = results.statistical_significance {
            if p_value < self.config.success_criteria.significance_level {
                // Find winning variant
                let winner = results.variant_results.iter()
                    .find(|(_, result)| result.is_winner)
                    .map(|(variant_id, _)| variant_id.clone());

                if let Some(winner_id) = winner {
                    if let Some(improvement) = self.config.success_criteria.expected_improvement {
                        let control_metrics = results.variant_results.values()
                            .find(|r| r.config.is_control)
                            .map(|r| &r.metrics);
                        let winner_metrics = results.variant_results.get(&winner_id)
                            .map(|r| &r.metrics);

                        if let (Some(control), Some(winner)) = (control_metrics, winner_metrics) {
                            let actual_improvement = (winner.average_success_rate() - control.average_success_rate())
                                / control.average_success_rate();

                            if actual_improvement >= improvement {
                                return ExperimentRecommendation::Deploy { variant_id: winner_id };
                            }
                        }
                    } else {
                        return ExperimentRecommendation::Deploy { variant_id: winner_id };
                    }
                }
            }
        }

        ExperimentRecommendation::Continue
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    pub experiment_id: String,
    pub experiment_name: String,
    pub status: crate::ab_testing::config::ExperimentStatus,
    pub variant_results: HashMap<String, VariantResult>,
    pub total_users: u32,
    pub statistical_significance: Option<f64>,
    pub recommendation: ExperimentRecommendation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantResult {
    pub config: VariantConfig,
    pub metrics: ExperimentMetrics,
    pub sample_size: u32,
    pub confidence_interval: (f64, f64),
    pub is_winner: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperimentRecommendation {
    Continue,
    Deploy { variant_id: String },
    Rollback,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentContext {
    pub experiment_id: String,
    pub variant_id: String,
    pub routing_policy: crate::ab_testing::config::RoutingPolicyConfig,
    pub user_assignment_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct InteractionMetrics {
    pub response_time_ms: u32,
    pub success: bool,
    pub user_rating: Option<u8>,
    pub cost: f64,
    pub error_message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct ExperimentRunner {
    pub experiments: HashMap<String, Experiment>,
    pub storage: Box<dyn ExperimentStorage>,
}

impl ExperimentRunner {
    pub fn new(storage: Box<dyn ExperimentStorage>) -> Self {
        Self {
            experiments: HashMap::new(),
            storage,
        }
    }

    pub async fn load_experiments(&mut self) -> Result<()> {
        let configs = self.storage.load_experiment_configs().await?;
        for config in configs {
            if config.can_run() {
                let experiment = Experiment::new(config)?;
                self.experiments.insert(experiment.config.id.clone(), experiment);
            }
        }
        Ok(())
    }

    pub fn get_variant_for_user(&mut self, experiment_id: &str, user_id: &str) -> Option<&Variant> {
        self.experiments
            .get_mut(experiment_id)
            .and_then(|exp| exp.assign_user(user_id))
    }

    pub fn record_interaction(&mut self, experiment_id: &str, user_id: &str, metrics: &InteractionMetrics) {
        if let Some(variant_id) = self.experiments
            .get_mut(experiment_id)
            .and_then(|exp| exp.user_assignments.get(user_id).cloned()) {
            if let Some(experiment) = self.experiments.get_mut(experiment_id) {
                experiment.record_interaction(user_id, &variant_id, metrics);
            }
        }
    }

    pub async fn save_results(&self) -> Result<()> {
        for experiment in self.experiments.values() {
            let results = experiment.get_results();
            self.storage.save_experiment_results(&results).await?;
        }
        Ok(())
    }
}