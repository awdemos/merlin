// src/server/openai_compatible.rs
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::server::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIModel {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIModelsResponse {
    pub object: String,
    pub data: Vec<OpenAIModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    pub r#type: String, // "function"
    pub function: OpenAIFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenAIResponseFormat {
    Text,
    JsonObject,
    #[serde(rename = "json_schema")]
    JsonSchema { json_schema: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
    pub tradeoff: Option<String>,           // NEW: "cost" | "latency" | "quality"
    pub timeout: Option<u32>,               // NEW: timeout in seconds
    pub default_model: Option<String>,        // NEW: fallback model
    pub tools: Option<Vec<OpenAITool>>,    // NEW: function calling
    pub response_format: Option<OpenAIResponseFormat>, // NEW: structured outputs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: Option<OpenAIUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIErrorResponse {
    pub error: OpenAIErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIErrorDetail {
    pub message: String,
    pub r#type: String,
    pub code: Option<String>,
}

pub async fn list_models(
    State(app_state): State<AppState>,
) -> Result<Json<OpenAIModelsResponse>, (StatusCode, Json<OpenAIErrorResponse>)> {
    let capability_loader = app_state.capability_loader.lock().await;
    let models = capability_loader.list_models();
    
    let openai_models: Vec<OpenAIModel> = models
        .into_iter()
        .map(|model_id| {
            let capabilities = capability_loader.get_capabilities_by_model(&model_id);
            OpenAIModel {
                id: model_id.clone(),
                object: "model".to_string(),
                created: 1640995200, // Placeholder timestamp
                owned_by: capabilities
                    .map(|cap| cap.provider.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
            }
        })
        .collect();

    Ok(Json(OpenAIModelsResponse {
        object: "list".to_string(),
        data: openai_models,
    }))
}

pub async fn chat_completions(
    State(app_state): State<AppState>,
    Json(request): Json<OpenAIChatRequest>,
) -> Result<Json<OpenAIChatResponse>, (StatusCode, Json<OpenAIErrorResponse>)> {
    // Extract the user message from the last message
    let user_message = request.messages
        .last()
        .and_then(|msg| {
            if msg.role == "user" {
                Some(msg.content.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "".to_string());

    // Get provider for the requested model
    let capability_loader = app_state.capability_loader.lock().await;
    let model_capabilities = capability_loader.get_capabilities_by_model(&request.model);
    
    let provider_name = model_capabilities
        .map(|cap| cap.provider.clone())
        .unwrap_or_else(|| "openai".to_string()); // Default fallback

    drop(capability_loader);

    // Create provider instance (simplified - in production, cache these)
    let provider_config = crate::providers::ProviderConfig {
        enabled: true,
        api_key: std::env::var("OPENAI_API_KEY").ok(),
        base_url: "https://api.openai.com/v1".to_string(),
        models: vec![request.model.clone()],
        default_model: request.model.clone(),
        custom_params: HashMap::new(),
    };

    let provider = app_state.provider_registry
        .create_provider(&provider_name, &provider_config)
        .map_err(|e| {
            tracing::error!("Failed to create provider {}: {}", provider_name, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OpenAIErrorResponse {
                    error: OpenAIErrorDetail {
                        message: format!("Failed to create provider: {}", e),
                        r#type: "internal_error".to_string(),
                        code: Some("provider_creation_failed".to_string()),
                    },
                }),
            )
        })?;

    // Call the provider
    let response_content = provider.chat(&user_message).await.map_err(|e| {
        tracing::error!("Provider chat failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OpenAIErrorResponse {
                error: OpenAIErrorDetail {
                    message: format!("Chat completion failed: {}", e),
                    r#type: "internal_error".to_string(),
                    code: Some("chat_failed".to_string()),
                },
            }),
        )
    })?;

    // Create OpenAI-compatible response
    let response_id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(Json(OpenAIChatResponse {
        id: response_id,
        object: "chat.completion".to_string(),
        created,
        model: request.model,
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: response_content,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Some(OpenAIUsage {
            prompt_tokens: 0, // TODO: Implement token counting
            completion_tokens: 0,
            total_tokens: 0,
        }),
    }))
}

pub fn create_openai_compatible_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
}