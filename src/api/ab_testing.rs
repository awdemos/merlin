// src/api/ab_testing.rs
use serde::{Deserialize, Serialize};
use crate::ab_testing::{
    config::{ExperimentConfig, VariantConfig, RoutingPolicyConfig, SuccessCriteria, MetricType, ExperimentStatus},
    experiment::{InteractionMetrics, ExperimentResults},
};

// === Experiment Management ===

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateExperimentRequest {
    pub name: String,
    pub description: String,
    pub variants: Vec<CreateVariantRequest>,
    pub traffic_allocation: f64,
    pub targeting_rules: Option<TargetingRulesRequest>,
    pub success_criteria: SuccessCriteriaRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVariantRequest {
    pub name: String,
    pub description: String,
    pub routing_policy: RoutingPolicyConfig,
    pub weight: f64,
    pub is_control: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetingRulesRequest {
    pub user_segments: Vec<String>,
    pub min_requests: Option<u32>,
    pub max_requests: Option<u32>,
    pub domains: Vec<String>,
    pub custom_attributes: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessCriteriaRequest {
    pub primary_metric: String,
    pub secondary_metrics: Vec<String>,
    pub significance_level: f64,
    pub min_sample_size: u32,
    pub expected_improvement: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateExperimentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub traffic_allocation: Option<f64>,
    pub targeting_rules: Option<TargetingRulesRequest>,
    pub success_criteria: Option<SuccessCriteriaRequest>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExperimentResponse {
    pub success: bool,
    pub experiment: Option<ExperimentConfig>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExperimentsListResponse {
    pub success: bool,
    pub experiments: Vec<ExperimentConfig>,
    pub message: String,
}

// === User Assignment ===

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAssignmentRequest {
    pub user_id: String,
    pub experiment_id: String,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAssignmentResponse {
    pub success: bool,
    pub variant_id: Option<String>,
    pub variant_name: Option<String>,
    pub experiment_id: String,
    pub message: String,
}

// === Metrics Recording ===

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordMetricsRequest {
    pub user_id: String,
    pub experiment_id: String,
    pub variant_id: String,
    pub response_time_ms: u32,
    pub success: bool,
    pub user_rating: Option<u8>,
    pub cost: f64,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordMetricsResponse {
    pub success: bool,
    pub message: String,
}

// === Experiment Results ===

#[derive(Debug, Serialize, Deserialize)]
pub struct ExperimentResultsResponse {
    pub success: bool,
    pub results: Option<ExperimentResults>,
    pub message: String,
}

// === API Conversion Functions ===

impl From<CreateExperimentRequest> for ExperimentConfig {
    fn from(req: CreateExperimentRequest) -> Self {
        let variants: Vec<VariantConfig> = req.variants.into_iter()
            .enumerate()
            .map(|(i, variant)| VariantConfig {
                id: format!("variant_{}", i + 1),
                name: variant.name,
                description: variant.description,
                routing_policy: variant.routing_policy,
                weight: variant.weight,
                is_control: variant.is_control,
            })
            .collect();

        let success_criteria = SuccessCriteria {
            primary_metric: parse_metric_type(&req.success_criteria.primary_metric),
            secondary_metrics: req.success_criteria.secondary_metrics.iter()
                .map(|s| parse_metric_type(s))
                .collect(),
            significance_level: req.success_criteria.significance_level,
            min_sample_size: req.success_criteria.min_sample_size,
            expected_improvement: req.success_criteria.expected_improvement,
        };

        let targeting_rules = req.targeting_rules.map(|rules| {
            let custom_attributes = if rules.custom_attributes.is_object() {
                rules.custom_attributes.as_object().unwrap().clone()
                    .into_iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else {
                std::collections::HashMap::new()
            };

            crate::ab_testing::config::TargetingRules {
                user_segments: rules.user_segments,
                min_requests: rules.min_requests,
                max_requests: rules.max_requests,
                domains: rules.domains,
                custom_attributes,
            }
        });

        let mut experiment_config = ExperimentConfig::new(
            uuid::Uuid::new_v4().to_string(),
            req.name,
            req.description,
            variants,
            req.traffic_allocation,
            success_criteria,
        );

        // Set targeting rules if provided
        experiment_config.targeting_rules = targeting_rules;

        experiment_config
    }
}

impl From<RecordMetricsRequest> for InteractionMetrics {
    fn from(req: RecordMetricsRequest) -> Self {
        InteractionMetrics {
            response_time_ms: req.response_time_ms,
            success: req.success,
            user_rating: req.user_rating,
            cost: req.cost,
            error_message: req.error_message,
            timestamp: chrono::Utc::now(),
        }
    }
}

fn parse_metric_type(metric_str: &str) -> MetricType {
    match metric_str.to_lowercase().as_str() {
        "response_time" => MetricType::ResponseTime,
        "success_rate" => MetricType::SuccessRate,
        "user_satisfaction" => MetricType::UserSatisfaction,
        "cost_efficiency" => MetricType::CostEfficiency,
        "error_rate" => MetricType::ErrorRate,
        "throughput" => MetricType::Throughput,
        _ => MetricType::SuccessRate, // Default fallback
    }
}

pub fn metric_type_to_string(metric_type: &MetricType) -> String {
    match metric_type {
        MetricType::ResponseTime => "response_time".to_string(),
        MetricType::SuccessRate => "success_rate".to_string(),
        MetricType::UserSatisfaction => "user_satisfaction".to_string(),
        MetricType::CostEfficiency => "cost_efficiency".to_string(),
        MetricType::ErrorRate => "error_rate".to_string(),
        MetricType::Throughput => "throughput".to_string(),
    }
}

pub fn experiment_status_to_string(status: &ExperimentStatus) -> String {
    match status {
        ExperimentStatus::Draft => "draft".to_string(),
        ExperimentStatus::Running => "running".to_string(),
        ExperimentStatus::Paused => "paused".to_string(),
        ExperimentStatus::Completed => "completed".to_string(),
        ExperimentStatus::Archived => "archived".to_string(),
    }
}

pub fn string_to_experiment_status(status_str: &str) -> Option<ExperimentStatus> {
    match status_str.to_lowercase().as_str() {
        "draft" => Some(ExperimentStatus::Draft),
        "running" => Some(ExperimentStatus::Running),
        "paused" => Some(ExperimentStatus::Paused),
        "completed" => Some(ExperimentStatus::Completed),
        "archived" => Some(ExperimentStatus::Archived),
        _ => None,
    }
}