// src/api/enhanced_model_select.rs
use serde::{Deserialize, Serialize};
use crate::ab_testing::ExperimentContext;

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedModelSelectRequest {
    pub prompt: String,
    pub models: Vec<String>,
    pub user_id: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f64>,
    pub domain_category: Option<String>,
    pub task_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedModelSelectResponse {
    pub request_id: String,
    pub selected_model: String,
    pub confidence: f64,
    pub reasoning: String,
    pub alternatives: Vec<crate::api::ModelAlternative>,
    pub experiment_context: ExperimentContext,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedInteractionRecord {
    pub request_id: String,
    pub user_id: String,
    pub model: String,
    pub experiment_context: ExperimentContext,
    pub response_time_ms: u32,
    pub success: bool,
    pub user_rating: Option<u8>,
    pub cost: f64,
    pub error_message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedInteractionResponse {
    pub success: bool,
    pub message: String,
    pub recorded_metrics: Option<EnhancedInteractionRecord>,
}