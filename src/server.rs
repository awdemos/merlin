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

use crate::config::{GovernanceConfig, MerlinConfig};
use crate::engine::RouterEngine;
use crate::governance::apps::{hash_token, App, Store};
use crate::governance::detect::{Detector, ScanVerdict};
use crate::governance::gate::{self, Sensitivity, TargetScope};
use crate::protocol::{
    AggLlmResponse, ContentBlock, LlmResponse, LlmResponseChunk, LlmResponseStreamEvent, Request,
};
use crate::translation::WireCodec;

pub type AppState = Arc<RouterEngine>;

#[derive(Clone)]
pub struct ServerState {
    engine: AppState,
    governance: Option<Governance>,
}

/// Governance runtime: config, token/usage store, sensitivity detector.
#[derive(Clone)]
pub struct Governance {
    pub config: GovernanceConfig,
    pub store: Arc<dyn Store>,
    pub detector: Detector,
}

pub async fn serve(addr: SocketAddr, config_path: Option<&str>) -> anyhow::Result<()> {
    let config = if let Some(path) = config_path {
        MerlinConfig::load_from_file(path)?
    } else {
        MerlinConfig::default()
    };
    let engine = Arc::new(RouterEngine::new(config.clone())?);

    let governance = if config.governance.enabled {
        let url = config
            .governance
            .redis_url
            .clone()
            .or_else(|| std::env::var("MERLIN_REDIS_URL").ok())
            .unwrap_or_else(|| "redis://127.0.0.1:6379".to_string());
        match crate::governance::apps::RedisStore::connect(&url).await {
            Ok(store) => Some(Governance {
                config: config.governance.clone(),
                store: Arc::new(store),
                detector: Detector::new(),
            }),
            Err(e) => {
                tracing::warn!(
                    "governance enabled but redis at {} unreachable ({}); \
                     governed requests will fail closed with 503",
                    url,
                    e
                );
                // Fail closed: keep governance enabled with a store that
                // errors on every op, so anonymous requests still get 401
                // and token-present requests get 503 store_error.
                Some(Governance {
                    config: config.governance.clone(),
                    store: Arc::new(crate::governance::apps::UnavailableStore),
                    detector: Detector::new(),
                })
            }
        }
    } else {
        None
    };

    let app = create_app_inner(engine, governance);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

pub fn create_app(engine: AppState) -> Router {
    create_app_inner(engine, None)
}

pub fn create_app_with_governance(engine: AppState, governance: Governance) -> Router {
    create_app_inner(engine, Some(governance))
}

fn create_app_inner(engine: AppState, governance: Option<Governance>) -> Router {
    let state = ServerState { engine, governance };

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

/// Outcome of the governance pipeline for one request.
#[derive(Debug, Clone)]
struct GovContext {
    app: App,
    sensitivity: Sensitivity,
    sensitivity_source: &'static str,
    eligible: Vec<String>,
}

/// Per-call gate trace attached to responses as `merlin_trace`.
fn trace_json(ctx: &GovContext, selected: Option<&str>) -> serde_json::Value {
    json!({
        "app": ctx.app.id,
        "sensitivity": match ctx.sensitivity {
            Sensitivity::Clean => "clean",
            Sensitivity::Sensitive => "sensitive",
            Sensitivity::Undetermined => "undetermined",
        },
        "sensitivity_source": ctx.sensitivity_source,
        "eligible": ctx.eligible,
        "selected": selected,
    })
}

fn denial_response(
    status: StatusCode,
    code: &str,
    message: &str,
    conflicts: Vec<String>,
) -> Response {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "error": {
                    "message": message,
                    "type": "merlin_error",
                    "code": code,
                    "conflicts": conflicts,
                }
            })
            .to_string(),
        ))
        .unwrap_or_default()
}

/// Run authn + quotas + gate for one request. Returns `Ok(None)` when
/// governance is disabled (anonymous, ungoverned). Returns `Err(response)`
/// when the request is denied — the handler must return it verbatim.
#[allow(clippy::result_large_err)]
async fn govern_request(
    state: &ServerState,
    headers: &HeaderMap,
    route_name: &str,
    prompt: &str,
) -> Result<Option<GovContext>, Response> {
    let gov = match &state.governance {
        Some(g) if g.config.enabled => g,
        _ => return Ok(None),
    };

    // 1. Authenticate.
    let raw_token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token);
    let raw_token = match raw_token {
        Some(t) => t,
        None => {
            state
                .engine
                .metrics()
                .record_governance_denial("unauthorized");
            return Err(denial_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "missing bearer token",
                Vec::new(),
            ));
        }
    };
    let hash = hash_token(raw_token);
    let (app_id, _token_id) = gov
        .store
        .app_for_token(&hash)
        .await
        .map_err(|e| {
            tracing::error!("token lookup failed: {e}");
            state
                .engine
                .metrics()
                .record_governance_denial("store_error");
            denial_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "store_error",
                "governance store unavailable",
                Vec::new(),
            )
        })?
        .ok_or_else(|| {
            state
                .engine
                .metrics()
                .record_governance_denial("unauthorized");
            denial_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "unknown or revoked token",
                Vec::new(),
            )
        })?;
    let app = gov.store.get_app(&app_id).await.map_err(|e| {
        tracing::error!("app lookup failed: {e}");
        state
            .engine
            .metrics()
            .record_governance_denial("store_error");
        denial_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "store_error",
            "governance store unavailable",
            Vec::new(),
        )
    })?;
    let app = match app {
        Some(app) => app,
        None => {
            state
                .engine
                .metrics()
                .record_governance_denial("unauthorized");
            return Err(denial_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "token references unknown app",
                Vec::new(),
            ));
        }
    };

    // Route allowlist.
    if let Some(allowed) = &app.policy.allowed_routes {
        if !allowed.iter().any(|r| r == route_name) {
            state
                .engine
                .metrics()
                .record_governance_denial("policy_denied");
            return Err(denial_response(
                StatusCode::FORBIDDEN,
                "policy_denied",
                &format!("route '{route_name}' not allowed for app '{}'", app.id),
                vec![format!("route '{route_name}' not in allowed_routes")],
            ));
        }
    }

    // 2. Quotas.
    if let Some(max_rpm) = app.policy.max_rpm {
        // RPM is consumed before the gate so denied requests still count
        // against the app's budget (abuse protection).
        let minute = chrono::Utc::now().timestamp() / 60;
        let count = gov.store.incr_rpm(&app.id, minute).await.map_err(|e| {
            tracing::error!("rpm counter failed: {e}");
            state
                .engine
                .metrics()
                .record_governance_denial("store_error");
            denial_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "store_error",
                "governance store unavailable",
                Vec::new(),
            )
        })?;
        if count > max_rpm as u64 {
            state
                .engine
                .metrics()
                .record_governance_denial("rate_exceeded");
            return Err(denial_response(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_exceeded",
                "per-app request rate exceeded",
                Vec::new(),
            ));
        }
    }
    if let Some(cap) = app.policy.max_spend_usd_per_day {
        let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let spent = gov.store.spend(&app.id, &day).await.map_err(|e| {
            tracing::error!("spend lookup failed: {e}");
            state
                .engine
                .metrics()
                .record_governance_denial("store_error");
            denial_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "store_error",
                "governance store unavailable",
                Vec::new(),
            )
        })?;
        if spent >= cap {
            state
                .engine
                .metrics()
                .record_governance_denial("budget_exhausted");
            return Err(denial_response(
                StatusCode::TOO_MANY_REQUESTS,
                "budget_exhausted",
                "per-app daily spend cap reached",
                Vec::new(),
            ));
        }
    }

    // 3. Sensitivity: sidecar first, local detector fallback. Non-text
    // content (images, tool results) is not scanned; with no extractable
    // text we cannot determine sensitivity, so fail closed.
    let trimmed = prompt.trim();
    let (sensitivity, source) = if trimmed.is_empty() {
        (Sensitivity::Undetermined, "no_text")
    } else {
        match state.engine.prompt_features(trimmed).await {
            Some(features) => {
                if features.is_sensitive >= gov.config.sensitivity_threshold {
                    (Sensitivity::Sensitive, "sidecar")
                } else {
                    (Sensitivity::Clean, "sidecar")
                }
            }
            None => match gov.detector.scan(trimmed) {
                ScanVerdict::Sensitive => (Sensitivity::Sensitive, "local"),
                ScanVerdict::Clean => (Sensitivity::Clean, "local_degraded"),
                ScanVerdict::Undetermined => (Sensitivity::Undetermined, "undetermined"),
            },
        }
    };

    // 4. Gate.
    let scope: Vec<TargetScope> = state
        .engine
        .route_scope(route_name)
        .into_iter()
        .map(|name| TargetScope {
            local: state.engine.target_locality(&name),
            priced: state.engine.target_price(&name).is_some(),
            name,
        })
        .collect();
    let eligible = match gate::evaluate(&app.policy, sensitivity, &scope) {
        Ok(eligible) => eligible,
        Err(denial) => {
            state
                .engine
                .metrics()
                .record_governance_denial("policy_denied");
            return Err(denial_response(
                StatusCode::FORBIDDEN,
                "policy_denied",
                "no eligible targets under app policy",
                denial.conflicts,
            ));
        }
    };

    Ok(Some(GovContext {
        app,
        sensitivity,
        sensitivity_source: source,
        eligible,
    }))
}

/// Charge a completed call against the app's daily budget.
async fn charge_spend(state: &ServerState, ctx: Option<&GovContext>, agg: &AggLlmResponse) {
    let (Some(ctx), Some(target_name)) = (ctx, agg.preservation.selected_target.as_ref()) else {
        return;
    };
    // Only track spend when a cap is configured; unpriced targets and
    // uncapped apps skip the store write entirely.
    if ctx.app.policy.max_spend_usd_per_day.is_none() {
        return;
    }
    let Some((price_in, price_out)) = state.engine.target_price(target_name) else {
        return;
    };
    let Some(gov) = state.governance.as_ref() else {
        return;
    };
    let cost = (agg.usage.prompt_tokens as f64) * price_in / 1_000_000.0
        + (agg.usage.completion_tokens as f64) * price_out / 1_000_000.0;
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if let Err(e) = gov.store.add_spend(&ctx.app.id, &day, cost).await {
        tracing::warn!("failed to charge spend for app {}: {e}", ctx.app.id);
    }
}

/// Charge spend and attach the governance trace to an aggregate response.
async fn aggregate_response(
    state: &ServerState,
    gov: Option<&GovContext>,
    agg: &AggLlmResponse,
    encode: impl Fn(&AggLlmResponse) -> serde_json::Value,
) -> Response {
    charge_spend(state, gov, agg).await;
    let mut value = encode(agg);
    if let Some(g) = gov {
        value.as_object_mut().expect("object").insert(
            "merlin_trace".to_string(),
            trace_json(g, agg.preservation.selected_target.as_deref()),
        );
    }
    Json(value).into_response()
}

/// Map a governed-execution failure: engine-level policy denials get the
/// 403 denial contract (and a metric); anything else is a generic error.
fn governed_error_response(state: &ServerState, status: StatusCode, e: &anyhow::Error) -> Response {
    if let Some(ge) = e.downcast_ref::<crate::engine::GovernanceError>() {
        state
            .engine
            .metrics()
            .record_governance_denial("policy_denied");
        return denial_response(
            StatusCode::FORBIDDEN,
            "policy_denied",
            &ge.message,
            vec![ge.message.clone()],
        );
    }
    error_response(status, &e.to_string())
}

async fn openai_chat_completions(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let codec = crate::translation::openai::OpenAiChatCodec;
    let request = match decode_request(body, &codec) {
        Ok(req) => req,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("bad request: {}", e)),
    };
    if let Err(reason) = state.engine.guard_check(&request).await {
        return error_response(
            StatusCode::BAD_REQUEST,
            &format!("blocked by prompt guard: {}", reason),
        );
    }

    let route_name = infer_route(
        &state.engine,
        request.llm_request.model.as_deref().unwrap_or(""),
    );
    let prompt = request.llm_request.prompt_text();
    let gov = match govern_request(&state, &headers, &route_name, &prompt).await {
        Ok(g) => g,
        Err(response) => return response,
    };
    let eligible = gov.as_ref().map(|g| g.eligible.clone());

    let stream = request.llm_request.stream;
    match state
        .engine
        .execute_governed(&route_name, request, eligible.as_deref())
        .await
    {
        Ok(LlmResponse::Aggregate(agg)) => {
            aggregate_response(&state, gov.as_ref(), &agg, openai_aggregate_response).await
        }
        Ok(LlmResponse::Stream(s)) => {
            if stream {
                // v1: SSE passthrough is not spend-accounted (documented limitation).
                sse_stream(s, &codec).into_response()
            } else {
                match crate::engine::aggregate_stream(s).await {
                    Ok(agg) => {
                        aggregate_response(&state, gov.as_ref(), &agg, openai_aggregate_response)
                            .await
                    }
                    Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
                }
            }
        }
        Err(e) => governed_error_response(&state, StatusCode::BAD_GATEWAY, &e),
    }
}

async fn anthropic_messages(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let codec = crate::translation::anthropic::AnthropicMessagesCodec;
    let request = match decode_request(body, &codec) {
        Ok(req) => req,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("bad request: {}", e)),
    };
    if let Err(reason) = state.engine.guard_check(&request).await {
        return error_response(
            StatusCode::BAD_REQUEST,
            &format!("blocked by prompt guard: {}", reason),
        );
    }

    let route_name = infer_route(
        &state.engine,
        request.llm_request.model.as_deref().unwrap_or(""),
    );
    let prompt = request.llm_request.prompt_text();
    let gov = match govern_request(&state, &headers, &route_name, &prompt).await {
        Ok(g) => g,
        Err(response) => return response,
    };
    let eligible = gov.as_ref().map(|g| g.eligible.clone());

    let stream = request.llm_request.stream;
    match state
        .engine
        .execute_governed(&route_name, request, eligible.as_deref())
        .await
    {
        Ok(LlmResponse::Aggregate(agg)) => {
            aggregate_response(&state, gov.as_ref(), &agg, anthropic_aggregate_response).await
        }
        Ok(LlmResponse::Stream(s)) => {
            if stream {
                // v1: SSE passthrough is not spend-accounted (documented limitation).
                sse_stream(s, &codec).into_response()
            } else {
                match crate::engine::aggregate_stream(s).await {
                    Ok(agg) => {
                        aggregate_response(&state, gov.as_ref(), &agg, anthropic_aggregate_response)
                            .await
                    }
                    Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
                }
            }
        }
        Err(e) => governed_error_response(&state, StatusCode::BAD_GATEWAY, &e),
    }
}

async fn decision_only(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let codec = crate::translation::openai::OpenAiChatCodec;
    let request = match decode_request(body, &codec) {
        Ok(req) => req,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("bad request: {}", e)),
    };
    if let Err(reason) = state.engine.guard_check(&request).await {
        return error_response(
            StatusCode::BAD_REQUEST,
            &format!("blocked by prompt guard: {}", reason),
        );
    }
    let route_name = infer_route(
        &state.engine,
        request.llm_request.model.as_deref().unwrap_or(""),
    );
    let prompt = request.llm_request.prompt_text();
    let gov = match govern_request(&state, &headers, &route_name, &prompt).await {
        Ok(g) => g,
        Err(response) => return response,
    };
    let eligible = gov.as_ref().map(|g| g.eligible.clone());

    match state
        .engine
        .decide_governed(&route_name, request, eligible.as_deref())
        .await
    {
        Ok(outcome) => {
            let mut value = json!({
                "selected": outcome.target.name,
                "client": outcome.target.client,
                "model": outcome.target.model_id,
                "fallbacks": outcome.fallback_chain,
            });
            if let Some(g) = &gov {
                value.as_object_mut().expect("object").insert(
                    "merlin_trace".to_string(),
                    trace_json(g, Some(&outcome.target.name)),
                );
            }
            Json(value).into_response()
        }
        Err(e) => governed_error_response(&state, StatusCode::BAD_REQUEST, &e),
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
    let content = if text.is_empty() { "" } else { text.as_str() };
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
                "content": content,
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
    let events = stream
        .inner
        .filter_map(move |event: LlmResponseStreamEvent| {
            let data = match event.chunk {
                LlmResponseChunk::ContentDelta {
                    block: ContentBlock::Text { text },
                } => Some(json!({
                    "id": "chatcmpl-merlin",
                    "object": "chat.completion.chunk",
                    "created": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    "model": "merlin",
                    "choices": [{"delta": {"content": text}, "index": 0, "finish_reason": null}]
                })),
                LlmResponseChunk::ContentDelta { .. } => None,
                LlmResponseChunk::ToolCallDelta { calls } => Some(json!({
                    "id": "chatcmpl-merlin",
                    "object": "chat.completion.chunk",
                    "created": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    "model": "merlin",
                    "choices": [{"delta": {"tool_calls": calls}, "index": 0, "finish_reason": null}],
                })),
                LlmResponseChunk::Done => Some(json!({
                    "id": "chatcmpl-merlin",
                    "object": "chat.completion.chunk",
                    "created": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    "model": "merlin",
                    "choices": [{"delta": {}, "index": 0, "finish_reason": "stop"}]
                })),
                LlmResponseChunk::Progress | LlmResponseChunk::Usage { .. } => None,
            };
            futures::future::ready(data.map(|d| {
                Ok(axum::response::sse::Event::default().data(d.to_string()))
            }))
        })
        .chain(futures::stream::once(async {
            Ok(axum::response::sse::Event::default().data("[DONE]"))
        }));
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
        .body(Body::from(
            json!({
                "error": {
                    "message": message,
                    "type": "merlin_error",
                    "code": status.as_u16(),
                }
            })
            .to_string(),
        ))
        .unwrap_or_default()
}
