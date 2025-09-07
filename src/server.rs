use axum::{
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

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
