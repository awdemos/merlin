// src/server/enhanced_model_select.rs
use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use crate::api::enhanced_model_select::*;
use crate::server::AppState;
use anyhow::Result;

pub fn create_enhanced_model_select_routes() -> Router<AppState> {
    Router::new()
        .route("/enhancedModelSelect", post(enhanced_model_select))
        .route("/enhancedModelSelect/:request_id/feedback", post(record_enhanced_feedback))
}

async fn enhanced_model_select(
    State(app_state): State<AppState>,
    Json(request): Json<EnhancedModelSelectRequest>,
) -> Result<Json<EnhancedModelSelectResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    // Validate request
    if request.prompt.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "Prompt cannot be empty".to_string(),
            }),
        ));
    }

    if request.models.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "At least one model must be specified".to_string(),
            }),
        ));
    }

    if request.user_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "User ID cannot be empty".to_string(),
            }),
        ));
    }

    // Convert to standard ModelSelectRequest for compatibility
    let messages = vec![crate::api::Message {
        role: "user".to_string(),
        content: request.prompt.clone(),
    }];

    let standard_request = crate::api::ModelSelectRequest {
        messages,
        models: request.models.clone(),
        preferences: None,
        session_id: None,
    };

    // Get user preferences if available
    let user_preferences = {
        let manager = &app_state.preference_server_state.preference_manager;
        manager.get_preferences(&request.user_id).await.ok()
    };

    // Use enhanced model selector
    let mut selector = crate::ab_testing::EnhancedModelSelector::new(app_state.experiment_runner.clone()).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(crate::server::ErrorResponse {
                    error: format!("Failed to create enhanced model selector: {}", e),
                }),
            )
        })?;

    let (model_response, experiment_context) = selector
        .select_model_with_ab_testing(&standard_request, user_preferences.as_ref(), &request.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(crate::server::ErrorResponse {
                    error: format!("Model selection failed: {}", e),
                }),
            )
        })?;

    // Create enhanced response
    let enhanced_response = EnhancedModelSelectResponse {
        request_id: uuid::Uuid::new_v4().to_string(),
        selected_model: model_response.recommended_model,
        confidence: model_response.confidence,
        reasoning: model_response.reasoning,
        alternatives: model_response.alternatives,
        experiment_context,
        metadata: None,
    };

    // Store the selection for later feedback recording
    // In a real implementation, you'd store this in a database or cache
    let feedback_storage = app_state.feedback_processor.lock().await;
    // Note: You might want to extend the feedback processor to handle enhanced selections

    Ok(Json(enhanced_response))
}

async fn record_enhanced_feedback(
    State(app_state): State<AppState>,
    Path(request_id): Path<String>,
    Json(interaction): Json<EnhancedInteractionRecord>,
) -> Result<Json<EnhancedInteractionResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    // Validate request
    if interaction.request_id != request_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "Request ID in path and body must match".to_string(),
            }),
        ));
    }

    if interaction.user_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "User ID cannot be empty".to_string(),
            }),
        ));
    }

    if interaction.model.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "Model cannot be empty".to_string(),
            }),
        ));
    }

    // Validate rating range
    if let Some(rating) = interaction.user_rating {
        if rating < 1 || rating > 5 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(crate::server::ErrorResponse {
                    error: "Rating must be between 1 and 5".to_string(),
                }),
            ));
        }
    }

    // Clone values before moving
    let user_id = interaction.user_id.clone();
    let model = interaction.model.clone();
    let experiment_context = interaction.experiment_context.clone();
    let response_time_ms = interaction.response_time_ms;
    let success = interaction.success;
    let user_rating = interaction.user_rating;
    let cost = interaction.cost;
    let error_message = interaction.error_message.clone();

    // Use enhanced model selector to record the interaction
    let mut selector = crate::ab_testing::EnhancedModelSelector::new(app_state.experiment_runner.clone()).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(crate::server::ErrorResponse {
                    error: format!("Failed to create enhanced model selector: {}", e),
                }),
            )
        })?;

    selector
        .record_interaction(
            &user_id,
            &model,
            &experiment_context,
            response_time_ms,
            success,
            user_rating,
            cost,
            error_message,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(crate::server::ErrorResponse {
                    error: format!("Failed to record interaction: {}", e),
                }),
            )
        })?;

    Ok(Json(EnhancedInteractionResponse {
        success: true,
        message: "Enhanced interaction recorded successfully".to_string(),
        recorded_metrics: Some(interaction),
    }))
}