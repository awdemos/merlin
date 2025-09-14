use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::api::OptimizationTarget;

// Re-export the preference models
pub use crate::preferences::models::{UserPreferences, UserStats, PromptInteractionFeatures, UserInteraction, PreferenceUpdateRequest};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserPreferenceRequest {
    pub user_id: String,
    pub optimize_for: Option<OptimizationTarget>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub custom_weights: Option<HashMap<String, f32>>,
    pub preferred_models: Option<Vec<String>>,
    pub excluded_models: Option<Vec<String>>,
    pub learning_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserPreferenceRequest {
    pub optimize_for: Option<OptimizationTarget>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub custom_weights: Option<HashMap<String, f32>>,
    pub preferred_models: Option<Vec<String>>,
    pub excluded_models: Option<Vec<String>>,
    pub learning_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPreferenceResponse {
    pub success: bool,
    pub preferences: Option<UserPreferences>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInteractionRequest {
    pub session_id: String,
    pub model_used: String,
    pub rating: Option<u8>,
    pub feedback_type: Option<String>,
    pub response_time_ms: Option<u32>,
    pub cost: Option<f64>,
    pub prompt_features: PromptFeaturesRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptFeaturesRequest {
    pub domain_category: String,
    pub task_type: String,
    pub complexity_score: f64,
    pub estimated_tokens: u32,
    pub keywords: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInteractionResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserStatsResponse {
    pub success: bool,
    pub stats: Option<UserStats>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRecommendationsResponse {
    pub success: bool,
    pub recommendations: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteUserPreferenceResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListUsersResponse {
    pub success: bool,
    pub users: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchUserStatsRequest {
    pub user_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchUserStatsResponse {
    pub success: bool,
    pub user_stats: HashMap<String, UserStats>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreferenceSearchRequest {
    pub optimize_for: Option<OptimizationTarget>,
    pub learning_enabled: Option<bool>,
    pub min_interaction_count: Option<u32>,
    pub preferred_models: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreferenceSearchResponse {
    pub success: bool,
    pub user_ids: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LearningSettingsRequest {
    pub learning_rate: Option<f64>,
    pub confidence_threshold: Option<f64>,
    pub max_history_size: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LearningSettingsResponse {
    pub success: bool,
    pub settings: LearningSettingsRequest,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelScoreRequest {
    pub model_name: String,
    pub prompt_features: PromptFeaturesRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelScoreResponse {
    pub success: bool,
    pub model_name: String,
    pub score: f64,
    pub breakdown: HashMap<String, f64>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportPreferencesRequest {
    pub user_id: String,
    pub format: ExportFormat,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportPreferencesResponse {
    pub success: bool,
    pub data: String,
    pub format: ExportFormat,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportPreferencesRequest {
    pub user_id: String,
    pub data: String,
    pub format: ExportFormat,
    pub overwrite: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportPreferencesResponse {
    pub success: bool,
    pub message: String,
    pub imported_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreferenceValidationRequest {
    pub user_id: String,
    pub preferences: CreateUserPreferenceRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreferenceValidationResponse {
    pub success: bool,
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub message: String,
}