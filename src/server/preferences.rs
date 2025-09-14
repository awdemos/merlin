use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::preferences::PreferenceManager;
use crate::api::preferences::*;
use crate::server::AppState;
use crate::preferences::models::{OptimizationTarget as ModelsOptimizationTarget};
use crate::features::PromptFeatures;
use crate::api::{DomainCategory, TaskType};

// Helper function to convert between API and models OptimizationTarget
fn convert_optimization_target(api_target: &crate::api::OptimizationTarget) -> ModelsOptimizationTarget {
    match api_target {
        crate::api::OptimizationTarget::Quality => ModelsOptimizationTarget::Quality,
        crate::api::OptimizationTarget::Speed => ModelsOptimizationTarget::Speed,
        crate::api::OptimizationTarget::Cost => ModelsOptimizationTarget::Cost,
        crate::api::OptimizationTarget::Balanced => ModelsOptimizationTarget::Balanced,
    }
}

#[derive(Clone)]
pub struct PreferenceServerState {
    pub preference_manager: Arc<PreferenceManager>,
}

impl PreferenceServerState {
    pub async fn new() -> anyhow::Result<Self> {
        let preference_manager = Arc::new(PreferenceManager::new().await?);
        Ok(Self { preference_manager })
    }
}

impl AsRef<Arc<PreferenceManager>> for PreferenceServerState {
    fn as_ref(&self) -> &Arc<PreferenceManager> {
        &self.preference_manager
    }
}

// Helper function to extract the manager from any state containing Arc<PreferenceManager>
fn extract_manager<T>(state: &State<T>) -> Arc<PreferenceManager>
where
    T: AsRef<Arc<PreferenceManager>>,
{
    state.as_ref().clone()
}

// Helper function to extract the manager from AppState
fn extract_manager_from_app_state(state: &AppState) -> Arc<PreferenceManager> {
    state.preference_server_state.preference_manager.clone()
}

// === User Preference CRUD Endpoints ===

pub async fn create_user_preferences(
    State(state): State<PreferenceServerState>,
    Json(request): Json<CreateUserPreferenceRequest>,
) -> Result<Json<UserPreferenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Convert API request to internal preference update request
    let update_request = PreferenceUpdateRequest {
        user_id: request.user_id.clone(),
        optimize_for: request.optimize_for.as_ref().map(|t| convert_optimization_target(t)),
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        custom_weights: request.custom_weights,
        preferred_models: request.preferred_models,
        excluded_models: request.excluded_models,
        learning_enabled: request.learning_enabled,
    };

    match state.preference_manager.update_preferences(update_request).await {
        Ok(response) => Ok(Json(UserPreferenceResponse {
            success: response.success,
            preferences: response.preferences,
            message: response.message,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to create user preferences: {}", e),
            }),
        )),
    }
}

pub async fn get_user_preferences(
    State(state): State<PreferenceServerState>,
    Path(user_id): Path<String>,
) -> Result<Json<UserPreferenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.preference_manager.get_preferences(&user_id).await {
        Ok(preferences) => Ok(Json(UserPreferenceResponse {
            success: true,
            preferences: Some(preferences),
            message: "Preferences retrieved successfully".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to retrieve user preferences: {}", e),
            }),
        )),
    }
}

pub async fn update_user_preferences(
    State(state): State<PreferenceServerState>,
    Path(user_id): Path<String>,
    Json(request): Json<UpdateUserPreferenceRequest>,
) -> Result<Json<UserPreferenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let update_request = PreferenceUpdateRequest {
        user_id: user_id.clone(),
        optimize_for: request.optimize_for.as_ref().map(|t| convert_optimization_target(t)),
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        custom_weights: request.custom_weights,
        preferred_models: request.preferred_models,
        excluded_models: request.excluded_models,
        learning_enabled: request.learning_enabled,
    };

    match state.preference_manager.update_preferences(update_request).await {
        Ok(response) => Ok(Json(UserPreferenceResponse {
            success: response.success,
            preferences: response.preferences,
            message: response.message,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to update user preferences: {}", e),
            }),
        )),
    }
}

pub async fn delete_user_preferences(
    State(state): State<PreferenceServerState>,
    Path(user_id): Path<String>,
) -> Result<Json<DeleteUserPreferenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.preference_manager.delete_user_preferences(&user_id).await {
        Ok(deleted) => Ok(Json(DeleteUserPreferenceResponse {
            success: deleted,
            message: if deleted {
                "User preferences deleted successfully".to_string()
            } else {
                "User preferences not found".to_string()
            },
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to delete user preferences: {}", e),
            }),
        )),
    }
}

pub async fn list_users(
    State(state): State<PreferenceServerState>,
) -> Result<Json<ListUsersResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.preference_manager.get_all_users().await {
        Ok(users) => Ok(Json(ListUsersResponse {
            success: true,
            users,
            message: "Users retrieved successfully".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to retrieve users: {}", e),
            }),
        )),
    }
}

// === User Interaction Endpoints ===

pub async fn record_user_interaction(
    State(state): State<PreferenceServerState>,
    Path(user_id): Path<String>,
    Json(request): Json<UserInteractionRequest>,
) -> Result<Json<UserInteractionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Convert API request to internal user interaction
    let interaction = UserInteraction {
        session_id: request.session_id,
        timestamp: chrono::Utc::now(),
        prompt_features: PromptInteractionFeatures {
            domain_category: request.prompt_features.domain_category,
            task_type: request.prompt_features.task_type,
            complexity_score: request.prompt_features.complexity_score,
            estimated_tokens: request.prompt_features.estimated_tokens,
            keywords: request.prompt_features.keywords,
        },
        model_used: request.model_used,
        rating: request.rating,
        feedback_type: request.feedback_type,
        response_time_ms: request.response_time_ms,
        cost: request.cost,
    };

    match state.preference_manager.record_user_interaction(&user_id, interaction).await {
        Ok(_) => Ok(Json(UserInteractionResponse {
            success: true,
            message: "User interaction recorded successfully".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to record user interaction: {}", e),
            }),
        )),
    }
}

// === User Analytics Endpoints ===

pub async fn get_user_stats(
    State(state): State<PreferenceServerState>,
    Path(user_id): Path<String>,
) -> Result<Json<UserStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.preference_manager.get_user_stats(&user_id).await {
        Ok(stats) => Ok(Json(UserStatsResponse {
            success: true,
            stats: Some(stats),
            message: "User stats retrieved successfully".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to retrieve user stats: {}", e),
            }),
        )),
    }
}

pub async fn get_user_recommendations(
    State(state): State<PreferenceServerState>,
    Path(user_id): Path<String>,
) -> Result<Json<UserRecommendationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.preference_manager.get_recommendations(&user_id).await {
        Ok(recommendations) => Ok(Json(UserRecommendationsResponse {
            success: true,
            recommendations,
            message: "User recommendations retrieved successfully".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to retrieve user recommendations: {}", e),
            }),
        )),
    }
}

pub async fn batch_get_user_stats(
    State(state): State<PreferenceServerState>,
    Json(request): Json<BatchUserStatsRequest>,
) -> Result<Json<BatchUserStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut user_stats = HashMap::new();
    let mut failed_users = Vec::new();

    for user_id in &request.user_ids {
        match state.preference_manager.get_user_stats(user_id).await {
            Ok(stats) => {
                user_stats.insert(user_id.clone(), stats);
            },
            Err(e) => {
                failed_users.push((user_id.clone(), e.to_string()));
            }
        }
    }

    Ok(Json(BatchUserStatsResponse {
        success: true,
        user_stats: user_stats.clone(),
        message: if failed_users.is_empty() {
            "Batch user stats retrieved successfully".to_string()
        } else {
            format!("Retrieved stats for {} users, {} failed", user_stats.len(), failed_users.len())
        },
    }))
}

// === Advanced Features Endpoints ===

pub async fn search_preferences(
    State(state): State<PreferenceServerState>,
    Query(params): Query<PreferenceSearchRequest>,
) -> Result<Json<PreferenceSearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.preference_manager.get_all_users().await {
        Ok(all_users) => {
            let mut matching_users = Vec::new();

            for user_id in all_users {
                if let Ok(preferences) = state.preference_manager.get_preferences(&user_id).await {
                    let mut matches = true;

                    // Check optimization target
                    if let Some(target) = &params.optimize_for {
                        let models_target = convert_optimization_target(target);
                        if preferences.optimize_for != models_target {
                            matches = false;
                        }
                    }

                    // Check learning enabled
                    if let Some(enabled) = &params.learning_enabled {
                        if preferences.learning_enabled != *enabled {
                            matches = false;
                        }
                    }

                    // Check interaction count
                    if let Some(min_count) = &params.min_interaction_count {
                        if preferences.interaction_history.len() < *min_count as usize {
                            matches = false;
                        }
                    }

                    // Check preferred models
                    if let Some(models) = &params.preferred_models {
                        let has_all_preferred = models.iter().all(|model| {
                            preferences.preferred_models.contains(model)
                        });
                        if !has_all_preferred {
                            matches = false;
                        }
                    }

                    if matches {
                        matching_users.push(user_id);
                    }
                }
            }

            Ok(Json(PreferenceSearchResponse {
                success: true,
                user_ids: matching_users.clone(),
                message: format!("Found {} matching users", matching_users.len()),
            }))
        },
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to search preferences: {}", e),
            }),
        )),
    }
}

pub async fn calculate_model_score(
    State(state): State<PreferenceServerState>,
    Path(user_id): Path<String>,
    Json(request): Json<ModelScoreRequest>,
) -> Result<Json<ModelScoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get user preferences
    let preferences = match state.preference_manager.get_preferences(&user_id).await {
        Ok(prefs) => prefs,
        Err(e) => return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get user preferences: {}", e),
            }),
        )),
    };

    // Convert prompt features
    let prompt_features = PromptFeatures {
        domain_category: match request.prompt_features.domain_category.as_str() {
            "general" => DomainCategory::General,
            "technical" => DomainCategory::Technical,
            "creative" => DomainCategory::Creative,
            "analytical" => DomainCategory::Analytical,
            "mathematical" => DomainCategory::Mathematical,
            "code_generation" => DomainCategory::CodeGeneration,
            "translation" => DomainCategory::Translation,
            "summarization" => DomainCategory::Summarization,
            _ => DomainCategory::General,
        },
        task_type: match request.prompt_features.task_type.as_str() {
            "question" => TaskType::Question,
            "instruction" => TaskType::Instruction,
            "conversation" => TaskType::Conversation,
            "completion" => TaskType::Completion,
            "analysis" => TaskType::Analysis,
            "generation" => TaskType::Generation,
            _ => TaskType::Instruction,
        },
        complexity_score: request.prompt_features.complexity_score,
        estimated_tokens: request.prompt_features.estimated_tokens,
        keyword_features: HashMap::new(), // Simplified for API
        length_features: 0.0,
        structural_features: 0.0,
    };

    // Calculate score
    let context_features = prompt_features.to_feature_vector();
    let base_score = preferences.calculate_model_preference_score(&request.model_name, &context_features);

    // Create score breakdown
    let mut breakdown = HashMap::new();
    breakdown.insert("base_score".to_string(), base_score);
    breakdown.insert("custom_weight".to_string(),
        preferences.custom_weights.get(&request.model_name).copied().unwrap_or(1.0) as f64);
    breakdown.insert("preferred_bonus".to_string(),
        if preferences.preferred_models.contains(&request.model_name) { 1.2 } else { 1.0 });
    breakdown.insert("excluded_penalty".to_string(),
        if preferences.excluded_models.contains(&request.model_name) { 0.1 } else { 1.0 });

    Ok(Json(ModelScoreResponse {
        success: true,
        model_name: request.model_name,
        score: base_score,
        breakdown,
        message: "Model score calculated successfully".to_string(),
    }))
}

// === Utility Endpoints ===

pub async fn export_preferences(
    State(state): State<PreferenceServerState>,
    Json(request): Json<ExportPreferencesRequest>,
) -> Result<Json<ExportPreferencesResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.preference_manager.get_preferences(&request.user_id).await {
        Ok(preferences) => {
            let data = match request.format {
                ExportFormat::Json => {
                    serde_json::to_string_pretty(&preferences).unwrap_or_else(|_| "{}".to_string())
                },
                ExportFormat::Csv => {
                    // Simple CSV export
                    let mut csv = String::new();
                    csv.push_str("user_id,optimize_for,max_tokens,temperature,learning_enabled,preferred_models,excluded_models\n");
                    csv.push_str(&format!("{},{},{},{},{},{:?},{:?}\n",
                        preferences.user_id,
                        serde_json::to_string(&preferences.optimize_for).unwrap_or_default(),
                        preferences.max_tokens,
                        preferences.temperature,
                        preferences.learning_enabled,
                        preferences.preferred_models,
                        preferences.excluded_models,
                    ));
                    csv
                },
            };

            Ok(Json(ExportPreferencesResponse {
                success: true,
                data,
                format: request.format,
                message: "Preferences exported successfully".to_string(),
            }))
        },
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to export preferences: {}", e),
            }),
        )),
    }
}

pub async fn validate_preferences(
    Json(request): Json<PreferenceValidationRequest>,
) -> Result<Json<PreferenceValidationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Validate user ID
    if request.user_id.is_empty() {
        errors.push("User ID cannot be empty".to_string());
    }

    // Validate temperature
    if let Some(temp) = request.preferences.temperature {
        if temp < 0.0 || temp > 2.0 {
            errors.push("Temperature must be between 0.0 and 2.0".to_string());
        }
    }

    // Validate max tokens
    if let Some(tokens) = request.preferences.max_tokens {
        if tokens < 1 || tokens > 32000 {
            errors.push("Max tokens must be between 1 and 32000".to_string());
        }
    }

    // Validate custom weights
    if let Some(ref weights) = request.preferences.custom_weights {
        for (model, weight) in weights {
            if *weight < 0.1 || *weight > 3.0 {
                errors.push(format!("Custom weight for model {} must be between 0.1 and 3.0", model));
            }
        }
    }

    // Warnings
    if request.preferences.preferred_models.as_ref().map_or(false, |models| models.len() > 10) {
        warnings.push("Having many preferred models may reduce selection effectiveness".to_string());
    }

    if request.preferences.excluded_models.as_ref().map_or(false, |models| models.len() > 5) {
        warnings.push("Having many excluded models may limit model selection options".to_string());
    }

    let is_valid = errors.is_empty();
    let message = if is_valid {
        "Preferences are valid".to_string()
    } else {
        "Preferences have validation errors".to_string()
    };

    Ok(Json(PreferenceValidationResponse {
        success: true,
        valid: is_valid,
        errors,
        warnings,
        message,
    }))
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}