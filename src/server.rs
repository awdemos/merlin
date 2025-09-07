use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{IntelligentModelSelector, FeedbackProcessor, api::{ModelSelectRequest, ModelSelectResponse, FeedbackRequest, FeedbackResponse}};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub max_tokens: Option<usize>,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub provider: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn create_server() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(handle_chat))
        .route("/metrics", get(get_metrics))
        // Note: modelSelect and feedback endpoints require state, use create_server_with_state instead
}

#[derive(Clone)]
pub struct AppState {
    pub model_selector: Arc<IntelligentModelSelector>,
    pub feedback_processor: Arc<tokio::sync::Mutex<FeedbackProcessor>>,
}

pub async fn create_server_with_state() -> anyhow::Result<Router> {
    let model_selector = Arc::new(
        IntelligentModelSelector::new().await?
    );
    
    let feedback_processor = Arc::new(
        tokio::sync::Mutex::new(FeedbackProcessor::new().await?)
    );

    let app_state = AppState {
        model_selector,
        feedback_processor,
    };
    
    Ok(Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(handle_chat))
        .route("/metrics", get(get_metrics))
        .route("/modelSelect", post(handle_model_select))
        .route("/feedback", post(handle_feedback))
        .with_state(app_state)
    )
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
    match app_state.model_selector.select_model(request).await {
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
