use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete, put},
    Router,
};
use serde::{Deserialize, Serialize};
use crate::preferences::models::{OptimizationTarget as ModelsOptimizationTarget};

fn convert_optimization_target(api_target: &crate::api::OptimizationTarget) -> ModelsOptimizationTarget {
    match api_target {
        crate::api::OptimizationTarget::Quality => ModelsOptimizationTarget::Quality,
        crate::api::OptimizationTarget::Speed => ModelsOptimizationTarget::Speed,
        crate::api::OptimizationTarget::Cost => ModelsOptimizationTarget::Cost,
        crate::api::OptimizationTarget::Balanced => ModelsOptimizationTarget::Balanced,
    }
}

use std::sync::Arc;
use crate::{IntelligentModelSelector, FeedbackProcessor, api::{ModelSelectRequest, ModelSelectResponse, FeedbackRequest, FeedbackResponse}};
use crate::providers::{MerlinConfig, ProviderRegistry, CapabilityLoader};

/// Request payload for the chat endpoint.
#[derive(Deserialize)]
pub struct ChatRequest {
    /// The prompt text to send to the LLM.
    pub prompt: String,
    /// Maximum tokens for the response (optional).
    pub max_tokens: Option<usize>,
}

/// Response from the chat endpoint containing the generated text.
#[derive(Serialize)]
pub struct ChatResponse {
    /// The generated response text.
    pub response: String,
    /// Name of the provider that generated the response.
    pub provider: String,
}

/// Error response payload for API errors.
#[derive(Serialize)]
pub struct ErrorResponse {
    /// Human-readable error message.
    pub error: String,
}

/// Creates a basic Axum router with health, chat, and metrics endpoints.
///
/// For full functionality including model selection and feedback,
/// use [`create_server_with_state`] instead.
pub fn create_server() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(handle_chat))
        .route("/metrics", get(get_metrics))
}

pub mod ab_testing;
pub mod enhanced_model_select;
pub mod preferences;
pub mod openai_compatible;

/// Shared application state for the Merlin server.
///
/// Contains all the services needed for request handling including
/// model selection, feedback processing, user preferences, and A/B testing.
#[derive(Clone)]
pub struct AppState {
    /// Intelligent model selector for choosing the best LLM.
    pub model_selector: Arc<tokio::sync::Mutex<IntelligentModelSelector>>,
    /// Feedback processor for handling user ratings.
    pub feedback_processor: Arc<tokio::sync::Mutex<FeedbackProcessor>>,
    /// User preference management state.
    pub preference_server_state: Arc<crate::server::preferences::PreferenceServerState>,
    /// A/B testing experiment runner.
    pub experiment_runner: Arc<tokio::sync::Mutex<crate::ab_testing::experiment::ExperimentRunner>>,
    pub provider_registry: Arc<ProviderRegistry>,
    pub capability_loader: Arc<tokio::sync::Mutex<CapabilityLoader>>,
}

impl AsRef<Arc<crate::preferences::PreferenceManager>> for AppState {
    fn as_ref(&self) -> &Arc<crate::preferences::PreferenceManager> {
        &self.preference_server_state.preference_manager
    }
}

/// Creates a fully-featured Axum router with all endpoints and state.
///
/// Initializes all services (model selector, feedback processor, preferences, A/B testing)
/// and returns a router ready for production use.
///
/// # Errors
///
/// Returns an error if initialization of any service fails.
pub async fn create_server_with_state() -> anyhow::Result<Router> {
    // Load configuration
    let config_path = std::env::var("MERLIN_CONFIG").unwrap_or_else(|_| "merlin.toml".to_string());
    let config = MerlinConfig::load_from_file(&config_path)
        .unwrap_or_else(|_| {
            tracing::warn!("Failed to load config from {}, using defaults", config_path);
            MerlinConfig::load_from_env().unwrap_or_else(|_| {
                tracing::warn!("Failed to load config from env, using hardcoded defaults");
                MerlinConfig::default()
            })
        });

    // Initialize provider registry
    let mut provider_registry = ProviderRegistry::new();
    provider_registry.register_default_factories();
    
    // Initialize capability loader
    let mut capability_loader = CapabilityLoader::new();
    let capabilities_file = config.routing.capabilities_file.as_deref().unwrap_or("capabilities.toml");
    if let Err(e) = capability_loader.load_from_file(capabilities_file).await {
        tracing::warn!("Failed to load capabilities from {}: {}", capabilities_file, e);
        // Use default capabilities as fallback
        capability_loader = CapabilityLoader::get_default_capabilities();
    }

    let capability_loader_arc = Arc::new(tokio::sync::Mutex::new(capability_loader));
    
    let model_selector = Arc::new(
        tokio::sync::Mutex::new(IntelligentModelSelector::new_with_capability_loader(
            capability_loader_arc.clone()
        ).await?)
    );

    let feedback_processor = Arc::new(
        tokio::sync::Mutex::new(FeedbackProcessor::new().await?)
    );

    let preference_server_state = Arc::new(
        crate::server::preferences::PreferenceServerState::new().await?
    );

    // Initialize experiment storage
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let experiment_storage: Box<dyn crate::ab_testing::ExperimentStorage> = match crate::ab_testing::storage::RedisExperimentStorage::new(&redis_url).await {
        Ok(storage) => Box::new(storage),
        Err(_) => {
            tracing::warn!("Redis not available, using in-memory storage for experiments");
            Box::new(crate::ab_testing::storage::InMemoryExperimentStorage::new())
        }
    };

    let experiment_runner = Arc::new(
        tokio::sync::Mutex::new(crate::ab_testing::experiment::ExperimentRunner::new(experiment_storage))
    );
    
    let app_state = AppState {
        model_selector,
        feedback_processor,
        preference_server_state,
        experiment_runner,
        provider_registry: Arc::new(provider_registry),
        capability_loader: capability_loader_arc.clone(),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(handle_chat))
        .route("/metrics", get(get_metrics))
        .route("/modelSelect", post(handle_model_select))
        .route("/feedback", post(handle_feedback))
        // User preference CRUD endpoints
        .route("/preferences/users", post(create_user_preferences_wrapper))
        .route("/preferences/users/:user_id", get(get_user_preferences_wrapper))
        .route("/preferences/users/:user_id", put(update_user_preferences_wrapper))
        .route("/preferences/users/:user_id", delete(delete_user_preferences_wrapper))
        .route("/preferences/users", get(list_users_wrapper))
        .route("/preferences/validate", post(validate_preferences_wrapper))
        // A/B testing endpoints
        .merge(crate::server::ab_testing::create_ab_testing_routes())
        // Enhanced model selection endpoints
        .merge(crate::server::enhanced_model_select::create_enhanced_model_select_routes())
        // OpenAI-compatible endpoints
        .merge(crate::server::openai_compatible::create_openai_compatible_routes())
        .with_state(app_state);

    Ok(app)
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn handle_chat() -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    // This would need dependency injection of the actual router
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Chat endpoint not implemented yet - needs DI setup".to_string(),
        }),
    ))
}

async fn get_metrics() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "metrics": "Not implemented yet"
    }))
}

async fn handle_model_select(
    State(app_state): State<AppState>,
    Json(request): Json<ModelSelectRequest>,
) -> Result<Json<ModelSelectResponse>, (StatusCode, Json<ErrorResponse>)> {
    match app_state.model_selector.lock().await.select_model(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

async fn handle_feedback(
    State(app_state): State<AppState>,
    Json(request): Json<FeedbackRequest>,
) -> Result<Json<FeedbackResponse>, (StatusCode, Json<ErrorResponse>)> {
    if request.rating < 1 || request.rating > 5 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Rating must be between 1 and 5".to_string(),
            }),
        ));
    }

    match app_state.feedback_processor.lock().await.process_feedback(&request).await {
        Ok(_) => Ok(Json(FeedbackResponse {
            success: true,
            message: "Feedback processed successfully".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to process feedback: {}", e),
            }),
        )),
    }
}

// === Preference API Wrappers ===

async fn create_user_preferences_wrapper(
    State(state): State<AppState>,
    Json(request): Json<crate::api::preferences::CreateUserPreferenceRequest>,
) -> Result<Json<crate::api::preferences::UserPreferenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = &state.preference_server_state.preference_manager;

    let update_request = crate::preferences::models::PreferenceUpdateRequest {
        user_id: request.user_id.clone(),
        optimize_for: request.optimize_for.as_ref().map(|t| convert_optimization_target(t)),
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        custom_weights: request.custom_weights,
        preferred_models: request.preferred_models,
        excluded_models: request.excluded_models,
        learning_enabled: request.learning_enabled,
    };

    match manager.update_preferences(update_request).await {
        Ok(response) => Ok(Json(crate::api::preferences::UserPreferenceResponse {
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

async fn get_user_preferences_wrapper(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<crate::api::preferences::UserPreferenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = &state.preference_server_state.preference_manager;

    match manager.get_preferences(&user_id).await {
        Ok(preferences) => Ok(Json(crate::api::preferences::UserPreferenceResponse {
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

async fn update_user_preferences_wrapper(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(request): Json<crate::api::preferences::UpdateUserPreferenceRequest>,
) -> Result<Json<crate::api::preferences::UserPreferenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = &state.preference_server_state.preference_manager;

    let update_request = crate::preferences::models::PreferenceUpdateRequest {
        user_id: user_id.clone(),
        optimize_for: request.optimize_for.as_ref().map(|t| convert_optimization_target(t)),
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        custom_weights: request.custom_weights,
        preferred_models: request.preferred_models,
        excluded_models: request.excluded_models,
        learning_enabled: request.learning_enabled,
    };

    match manager.update_preferences(update_request).await {
        Ok(response) => Ok(Json(crate::api::preferences::UserPreferenceResponse {
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

async fn delete_user_preferences_wrapper(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<crate::api::preferences::DeleteUserPreferenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = &state.preference_server_state.preference_manager;

    match manager.delete_user_preferences(&user_id).await {
        Ok(deleted) => Ok(Json(crate::api::preferences::DeleteUserPreferenceResponse {
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

async fn list_users_wrapper(
    State(state): State<AppState>,
) -> Result<Json<crate::api::preferences::ListUsersResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = &state.preference_server_state.preference_manager;

    match manager.get_all_users().await {
        Ok(users) => Ok(Json(crate::api::preferences::ListUsersResponse {
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

async fn validate_preferences_wrapper(
    Json(request): Json<crate::api::preferences::PreferenceValidationRequest>,
) -> Result<Json<crate::api::preferences::PreferenceValidationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut errors = Vec::new();
    let warnings = Vec::new();

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

    let is_valid = errors.is_empty();
    let message = if is_valid {
        "Preferences are valid".to_string()
    } else {
        "Preferences have validation errors".to_string()
    };

    Ok(Json(crate::api::preferences::PreferenceValidationResponse {
        success: true,
        valid: is_valid,
        errors,
        warnings,
        message,
    }))
}
