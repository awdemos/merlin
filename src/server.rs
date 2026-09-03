use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::config::MerlinConfig;
use crate::engine::RouterEngine;
use crate::protocol::{
    AggLlmResponse, ContentBlock, LlmResponse, LlmResponseChunk, LlmResponseStreamEvent, Request,
};
use crate::translation::WireCodec;

pub type AppState = Arc<RouterEngine>;

#[derive(Clone)]
pub struct ServerState {
    engine: AppState,
}

pub async fn serve(addr: SocketAddr, config_path: Option<&str>) -> anyhow::Result<()> {
    let config = if let Some(path) = config_path {
        MerlinConfig::load_from_file(path)?
    } else {
        MerlinConfig::default()
    };
    let engine = Arc::new(RouterEngine::new(config)?);

    let app = create_app(engine);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

pub fn create_app(engine: AppState) -> Router {
    let state = ServerState { engine };

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(openai_chat_completions))
        .route("/v1/messages", post(anthropic_messages))
        .route("/v1/decision", post(decision_only))
        .route("/v1/feedback", post(feedback))
        .route("/metrics", get(prometheus_metrics))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(TimeoutLayer::new(Duration::from_secs(300)))
        .with_state(state)
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    let terminate = async {
        #[cfg(unix)]
        {
            let mut sig = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler");
            sig.recv().await;
        }
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}

async fn list_models(State(state): State<ServerState>) -> Json<serde_json::Value> {
    Json(json!({
        "object": "list",
        "data": state.engine.route_models()
    }))
}

async fn prometheus_metrics(State(state): State<ServerState>) -> Response {
    match state.engine.metrics().encode() {
        Ok(text) => Response::builder()
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(Body::from(text))
            .unwrap_or_default(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("metrics error: {}", e)))
            .unwrap_or_default(),
    }
}

async fn openai_chat_completions(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    body: Bytes,
) -> Response {
    let codec = crate::translation::openai::OpenAiChatCodec;
    let request = match decode_request(body, &codec) {
        Ok(req) => req,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("bad request: {}", e)),
    };

    let route_name = infer_route(
        &state.engine,
        request.llm_request.model.as_deref().unwrap_or(""),
    );

    let stream = request.llm_request.stream;
    match state.engine.execute(&route_name, request).await {
        Ok(LlmResponse::Aggregate(agg)) => Json(openai_aggregate_response(&agg)).into_response(),
        Ok(LlmResponse::Stream(s)) => {
            if stream {
                sse_stream(s, &codec).into_response()
            } else {
                match crate::engine::aggregate_stream(s).await {
                    Ok(agg) => Json(openai_aggregate_response(&agg)).into_response(),
                    Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
                }
            }
        }
        Err(e) => error_response(StatusCode::BAD_GATEWAY, &e.to_string()),
    }
}

async fn anthropic_messages(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    body: Bytes,
) -> Response {
    let codec = crate::translation::anthropic::AnthropicMessagesCodec;
    let request = match decode_request(body, &codec) {
        Ok(req) => req,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("bad request: {}", e)),
    };

    let route_name = infer_route(
        &state.engine,
        request.llm_request.model.as_deref().unwrap_or(""),
    );
    let stream = request.llm_request.stream;

    match state.engine.execute(&route_name, request).await {
        Ok(LlmResponse::Aggregate(agg)) => Json(anthropic_aggregate_response(&agg)).into_response(),
        Ok(LlmResponse::Stream(s)) => {
            if stream {
                sse_stream(s, &codec).into_response()
            } else {
                match crate::engine::aggregate_stream(s).await {
                    Ok(agg) => Json(anthropic_aggregate_response(&agg)).into_response(),
                    Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
                }
            }
        }
        Err(e) => error_response(StatusCode::BAD_GATEWAY, &e.to_string()),
    }
}

async fn decision_only(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    body: Bytes,
) -> Response {
    let codec = crate::translation::openai::OpenAiChatCodec;
    let request = match decode_request(body, &codec) {
        Ok(req) => req,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("bad request: {}", e)),
    };
    let route_name = infer_route(
        &state.engine,
        request.llm_request.model.as_deref().unwrap_or(""),
    );
    match state.engine.decide(&route_name, request).await {
        Ok(outcome) => Json(json!({
            "selected": outcome.target.name,
            "client": outcome.target.client,
            "model": outcome.target.model_id,
            "fallbacks": outcome.fallback_chain,
        }))
        .into_response(),
        Err(e) => error_response(StatusCode::BAD_REQUEST, &e.to_string()),
    }
}

async fn feedback(
    State(_state): State<ServerState>,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    let route_name = payload
        .get("route")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let _target = payload.get("target").and_then(|v| v.as_str());
    let _reward = payload
        .get("reward")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let _request_id = payload.get("request_id").and_then(|v| v.as_str());
    tracing::info!("feedback received for route {}", route_name);
    Json(json!({"status": "ok"})).into_response()
}

fn infer_route(engine: &RouterEngine, model_id: &str) -> String {
    if engine.list_routes().contains(&model_id.to_string()) {
        model_id.to_string()
    } else {
        engine
            .list_routes()
            .into_iter()
            .next()
            .unwrap_or_else(|| "default".to_string())
    }
}

fn decode_request(body: Bytes, codec: &dyn WireCodec) -> anyhow::Result<Request> {
    let value: serde_json::Value = serde_json::from_slice(&body)?;
    codec.decode_request(value)
}

fn openai_aggregate_response(agg: &AggLlmResponse) -> serde_json::Value {
    let text = agg.assistant_text();
    let mut tool_calls = Vec::new();
    if let Some(calls) = &agg.tool_calls {
        for call in calls {
            tool_calls.push(json!({
                "id": call.id,
                "type": "function",
                "function": {
                    "name": call.function.name,
                    "arguments": call.function.arguments,
                }
            }));
        }
    }
    json!({
        "id": agg.id,
        "object": "chat.completion",
        "created": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        "model": agg.model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": if text.is_empty() { serde_json::Value::Null } else { json!(text) },
                "tool_calls": if tool_calls.is_empty() { serde_json::Value::Null } else { json!(tool_calls) },
            },
            "finish_reason": encode_stop_reason(agg.stop_reason),
        }],
        "usage": {
            "prompt_tokens": agg.usage.prompt_tokens,
            "completion_tokens": agg.usage.completion_tokens,
            "total_tokens": agg.usage.total_tokens,
        }
    })
}

fn anthropic_aggregate_response(agg: &AggLlmResponse) -> serde_json::Value {
    let text = agg.assistant_text();
    json!({
        "id": agg.id,
        "type": "message",
        "role": "assistant",
        "model": agg.model,
        "content": [{"type": "text", "text": text}],
        "stop_reason": match agg.stop_reason {
            crate::protocol::StopReason::Stop => serde_json::Value::String("end_turn".to_string()),
            crate::protocol::StopReason::Length => serde_json::Value::String("max_tokens".to_string()),
            crate::protocol::StopReason::ToolCalls => serde_json::Value::String("tool_use".to_string()),
            _ => serde_json::Value::Null,
        },
        "usage": {
            "input_tokens": agg.usage.prompt_tokens,
            "output_tokens": agg.usage.completion_tokens,
        }
    })
}

fn encode_stop_reason(reason: crate::protocol::StopReason) -> serde_json::Value {
    match reason {
        crate::protocol::StopReason::Stop => json!("stop"),
        crate::protocol::StopReason::Length => json!("length"),
        crate::protocol::StopReason::ToolCalls => json!("tool_calls"),
        crate::protocol::StopReason::ContentFilter => json!("content_filter"),
        crate::protocol::StopReason::Unknown => serde_json::Value::Null,
    }
}

fn sse_stream(
    stream: crate::protocol::LlmResponseStream,
    _codec: &dyn WireCodec,
) -> Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>
{
    let events = stream.inner.map(move |event: LlmResponseStreamEvent| {
        let data = match event.chunk {
            LlmResponseChunk::ContentDelta {
                block: ContentBlock::Text { text },
            } => json!({
                "choices": [{"delta": {"content": text}, "index": 0, "finish_reason": null}]
            }),
            LlmResponseChunk::ContentDelta { .. } => json!({}),
            LlmResponseChunk::ToolCallDelta { calls } => json!({
                "choices": [{"delta": {"tool_calls": calls}, "index": 0, "finish_reason": null}],
            }),
            LlmResponseChunk::Done => {
                json!({"choices": [{"delta": {}, "index": 0, "finish_reason": "stop"}]})
            }
            _ => json!({}),
        };
        Ok(axum::response::sse::Event::default().data(data.to_string()))
    });
    Sse::new(events).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text(""),
    )
}

fn error_response(status: StatusCode, message: &str) -> Response {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"error": message}).to_string()))
        .unwrap_or_default()
}
