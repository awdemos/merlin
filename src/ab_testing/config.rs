// src/ab_testing/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub variants: Vec<VariantConfig>,
    pub traffic_allocation: f64, // 0.0 to 1.0
    pub targeting_rules: Option<TargetingRules>,
    pub success_criteria: SuccessCriteria,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: ExperimentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub routing_policy: RoutingPolicyConfig,
    pub weight: f64, // Traffic allocation for this variant
    pub is_control: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "config")]
pub enum RoutingPolicyConfig {
    EpsilonGreedy { epsilon: f64 },
    ThompsonSampling { arms_count: usize },
    UpperConfidenceBound { confidence_level: f64 },
    Contextual { feature_dim: usize, learning_rate: f64 },
    Static { provider_index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetingRules {
    pub user_segments: Vec<String>,
    pub min_requests: Option<u32>,
    pub max_requests: Option<u32>,
    pub domains: Vec<String>,
    pub custom_attributes: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteria {
    pub primary_metric: MetricType,
    pub secondary_metrics: Vec<MetricType>,
    pub significance_level: f64, // e.g., 0.05 for 95% confidence
    pub min_sample_size: u32,
    pub expected_improvement: Option<f64>, // Minimum improvement to be considered successful
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    ResponseTime,
    SuccessRate,
    UserSatisfaction,
    CostEfficiency,
    ErrorRate,
    Throughput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Draft,
    Running,
    Paused,
    Completed,
    Archived,
}

impl ExperimentConfig {
    pub fn new(
        id: String,
        name: String,
        description: String,
        variants: Vec<VariantConfig>,
        traffic_allocation: f64,
        success_criteria: SuccessCriteria,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description,
            variants,
            traffic_allocation,
            targeting_rules: None,
            success_criteria,
            start_time: now,
            end_time: None,
            status: ExperimentStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate traffic allocation
        if self.traffic_allocation <= 0.0 || self.traffic_allocation > 1.0 {
            return Err("Traffic allocation must be between 0.0 and 1.0".to_string());
        }

        // Validate variants
        if self.variants.len() < 2 {
            return Err("At least 2 variants are required".to_string());
        }

        let control_count = self.variants.iter().filter(|v| v.is_control).count();
        if control_count != 1 {
            return Err("Exactly one variant must be marked as control".to_string());
        }

        let total_weight: f64 = self.variants.iter().map(|v| v.weight).sum();
        if (total_weight - 1.0).abs() > 0.001 {
            return Err("Variant weights must sum to 1.0".to_string());
        }

        // Validate success criteria
        if self.success_criteria.significance_level <= 0.0 || self.success_criteria.significance_level >= 1.0 {
            return Err("Significance level must be between 0.0 and 1.0".to_string());
        }

        if self.success_criteria.min_sample_size < 100 {
            return Err("Minimum sample size must be at least 100".to_string());
        }

        Ok(())
    }

    pub fn can_run(&self) -> bool {
        matches!(self.status, ExperimentStatus::Running) &&
        self.start_time <= Utc::now() &&
        self.end_time.map_or(true, |end| end > Utc::now())
    }
}