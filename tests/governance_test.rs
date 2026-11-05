//! Server-level governance: auth, quotas, gate, trace.

use merlin::config::{
    AlgorithmConfig, ClientConfig, GovernanceConfig, MerlinConfig, RouteConfig, TargetConfig,
};
use merlin::engine::RouterEngine;
use merlin::governance::apps::{App, AppPolicy, MemoryStore, Store};
use merlin::governance::detect::Detector;
use merlin::server::{create_app_with_governance, Governance};
use std::sync::Arc;

fn gov_config() -> GovernanceConfig {
    GovernanceConfig {
        enabled: true,
        redis_url: None,
        sensitivity_threshold: 0.7,
    }
}

async fn governed_app(policy: AppPolicy) -> (axum::Router, Arc<MemoryStore>) {
    let local_mock = wiremock::MockServer::start().await;
    let cloud_mock = wiremock::MockServer::start().await;
    for mock in [&local_mock, &cloud_mock] {
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
                    "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                                  "finish_reason": "stop"}],
                    "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
                })),
            )
            .mount(mock)
            .await;
    }

    let mut config = MerlinConfig::default();
    config.server.port = 0;
    config.clients.insert(
        "local".to_string(),
        ClientConfig {
            format: "openai_chat".to_string(),
            base_url: local_mock.uri(),
            api_key_env: None,
            max_retries: 0,
            forward_auth: false,
            extra_headers: Default::default(),
            timeout_ms: 5000,
        },
    );
    config.clients.insert(
        "cloud".to_string(),
        ClientConfig {
            format: "openai_chat".to_string(),
            base_url: cloud_mock.uri(),
            api_key_env: None,
            max_retries: 0,
            forward_auth: false,
            extra_headers: Default::default(),
            timeout_ms: 5000,
        },
    );
    config.targets.insert(
        "local_t".to_string(),
        TargetConfig {
            name: "local_t".to_string(),
            client: "local".to_string(),
            model: "local-model".to_string(),
            extra_body: None,
            price_input: None,
            price_output: None,
        },
    );
    config.targets.insert(
        "cloud_t".to_string(),
        TargetConfig {
            name: "cloud_t".to_string(),
            client: "cloud".to_string(),
            model: "cloud-model".to_string(),
            extra_body: None,
            price_input: Some(10.0),
            price_output: Some(20.0),
        },
    );
    config.routes.insert(
        "default".to_string(),
        RouteConfig {
            id: "default".to_string(),
            default_model: None,
            targets: vec!["local_t".to_string(), "cloud_t".to_string()],
            algorithm: AlgorithmConfig::EpsilonGreedy {
                targets: vec!["local_t".to_string(), "cloud_t".to_string()],
                epsilon: 0.0,
            },
        },
    );
    config.governance = gov_config();

    let engine = RouterEngine::new(config).unwrap();
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new("myapp", "My App");
    app.policy = policy;
    store.put_app(&app).await.unwrap();
    let gov = Governance {
        config: gov_config(),
        store: store.clone(),
        detector: Detector::new(),
    };
    (create_app_with_governance(Arc::new(engine), gov), store)
}

fn chat_request() -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::json!({
                "model": "default",
                "messages": [{"role": "user", "content": "hello"}]
            })
            .to_string(),
        ))
        .unwrap()
}

#[tokio::test]
async fn missing_token_gets_401() {
    use tower::ServiceExt;
    let (app, _store) = governed_app(AppPolicy::default()).await;
    let response = app.oneshot(chat_request()).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valid_token_gets_trace_and_success() {
    use tower::ServiceExt;
    let (app, store) = governed_app(AppPolicy::default()).await;
    let raw = merlin::governance::apps::mint_token();
    let hash = merlin::governance::apps::hash_token(&raw);
    store.put_token("myapp", "tid-1", &hash).await.unwrap();

    let request = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {raw}"))
        .body(axum::body::Body::from(
            serde_json::json!({
                "model": "default",
                "messages": [{"role": "user", "content": "hello"}]
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body["merlin_trace"].is_object(), "trace present: {body}");
    assert_eq!(body["merlin_trace"]["app"].as_str().unwrap(), "myapp");
}

#[tokio::test]
async fn over_rpm_gets_429() {
    use tower::ServiceExt;
    let (app, store) = governed_app(AppPolicy {
        max_rpm: Some(1),
        ..Default::default()
    })
    .await;
    let raw = merlin::governance::apps::mint_token();
    let hash = merlin::governance::apps::hash_token(&raw);
    store.put_token("myapp", "tid-1", &hash).await.unwrap();

    let req = || {
        axum::http::Request::builder()
            .method(axum::http::Method::POST)
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {raw}"))
            .body(axum::body::Body::from(
                serde_json::json!({
                    "model": "default",
                    "messages": [{"role": "user", "content": "hello"}]
                })
                .to_string(),
            ))
            .unwrap()
    };
    let first = app.clone().oneshot(req()).await.unwrap();
    assert_eq!(first.status(), axum::http::StatusCode::OK);
    let second = app.oneshot(req()).await.unwrap();
    assert_eq!(second.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn sensitive_request_denied_when_no_local_targets() {
    use tower::ServiceExt;
    // Inline config: the shared helper's wiremock upstreams are 127.0.0.1
    // (local per host_is_local), so a deny test needs a genuinely
    // non-local client. "cloud" points at example.com and is never
    // contacted: the gate denies before any upstream call.
    let mut config = MerlinConfig::default();
    config.server.port = 0;
    config.clients.insert(
        "cloud".to_string(),
        ClientConfig {
            format: "openai_chat".to_string(),
            base_url: "https://example.com".to_string(),
            api_key_env: None,
            max_retries: 0,
            forward_auth: false,
            extra_headers: Default::default(),
            timeout_ms: 5000,
        },
    );
    config.targets.insert(
        "cloud_t".to_string(),
        TargetConfig {
            name: "cloud_t".to_string(),
            client: "cloud".to_string(),
            model: "cloud-model".to_string(),
            extra_body: None,
            price_input: None,
            price_output: None,
        },
    );
    config.routes.insert(
        "default".to_string(),
        RouteConfig {
            id: "default".to_string(),
            default_model: None,
            targets: vec!["cloud_t".to_string()],
            algorithm: AlgorithmConfig::Passthrough {
                target: "cloud_t".to_string(),
            },
        },
    );
    config.governance = gov_config();

    let engine = RouterEngine::new(config).unwrap();
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new("myapp", "My App");
    // Raise-only policy: a sensitive request may only use local targets,
    // and this route has none — the gate must deny before any upstream call.
    app.policy = AppPolicy {
        sensitive_local_only: true,
        ..Default::default()
    };
    store.put_app(&app).await.unwrap();
    let raw = merlin::governance::apps::mint_token();
    let hash = merlin::governance::apps::hash_token(&raw);
    store.put_token("myapp", "tid-1", &hash).await.unwrap();

    let gov = Governance {
        config: gov_config(),
        store: store.clone(),
        detector: Detector::new(),
    };
    let app_router = create_app_with_governance(Arc::new(engine), gov);

    let request = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {raw}"))
        .body(axum::body::Body::from(
            serde_json::json!({
                "model": "default",
                "messages": [{"role": "user", "content": "my ssn is 123-45-6789"}]
            })
            .to_string(),
        ))
        .unwrap();
    let response = app_router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"]["code"], "policy_denied");
    assert!(body["error"]["conflicts"].is_array());
    assert!(!body["error"]["conflicts"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn unreachable_store_fails_closed_with_503() {
    use tower::ServiceExt;
    // Rebuild the router with an unavailable store to simulate Redis down
    // at startup.
    let mut config = MerlinConfig::default();
    config.server.port = 0;
    let mock = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                              "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
            })),
        )
        .mount(&mock)
        .await;
    config.clients.insert(
        "c".to_string(),
        ClientConfig {
            format: "openai_chat".to_string(),
            base_url: mock.uri(),
            api_key_env: None,
            max_retries: 0,
            forward_auth: false,
            extra_headers: Default::default(),
            timeout_ms: 5000,
        },
    );
    config.targets.insert(
        "t".to_string(),
        TargetConfig {
            name: "t".to_string(),
            client: "c".to_string(),
            model: "m".to_string(),
            extra_body: None,
            price_input: None,
            price_output: None,
        },
    );
    config.routes.insert(
        "default".to_string(),
        RouteConfig {
            id: "default".to_string(),
            default_model: None,
            targets: vec!["t".to_string()],
            algorithm: AlgorithmConfig::Passthrough {
                target: "t".to_string(),
            },
        },
    );
    let engine = RouterEngine::new(config).unwrap();
    let gov = Governance {
        config: gov_config(),
        store: Arc::new(merlin::governance::apps::UnavailableStore),
        detector: Detector::new(),
    };
    let app = create_app_with_governance(Arc::new(engine), gov);

    // No token → 401 (auth runs before any store op).
    let anon = app.clone().oneshot(chat_request()).await.unwrap();
    assert_eq!(anon.status(), axum::http::StatusCode::UNAUTHORIZED);

    // Any token → store error → 503, never 200.
    let raw = merlin::governance::apps::mint_token();
    let authed = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {raw}"))
        .body(axum::body::Body::from(
            serde_json::json!({"model": "default", "messages": [{"role": "user", "content": "hi"}]})
                .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(authed).await.unwrap();
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"]["code"], "store_error");
}

#[tokio::test]
async fn governance_disabled_allows_anonymous() {
    use tower::ServiceExt;
    let mut config = MerlinConfig::default();
    config.server.port = 0;
    let mock = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                              "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
            })),
        )
        .mount(&mock)
        .await;
    config.clients.insert(
        "c".to_string(),
        ClientConfig {
            format: "openai_chat".to_string(),
            base_url: mock.uri(),
            api_key_env: None,
            max_retries: 0,
            forward_auth: false,
            extra_headers: Default::default(),
            timeout_ms: 5000,
        },
    );
    config.targets.insert(
        "t".to_string(),
        TargetConfig {
            name: "t".to_string(),
            client: "c".to_string(),
            model: "m".to_string(),
            extra_body: None,
            price_input: None,
            price_output: None,
        },
    );
    config.routes.insert(
        "default".to_string(),
        RouteConfig {
            id: "default".to_string(),
            default_model: None,
            targets: vec!["t".to_string()],
            algorithm: AlgorithmConfig::Passthrough {
                target: "t".to_string(),
            },
        },
    );
    let engine = RouterEngine::new(config).unwrap();
    let response = merlin::server::create_app(Arc::new(engine))
        .oneshot(chat_request())
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn budget_exhausted_gets_429() {
    use tower::ServiceExt;
    let (app, store) = governed_app(AppPolicy {
        max_spend_usd_per_day: Some(1.0),
        ..Default::default()
    })
    .await;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    store.add_spend("myapp", &today, 1.0).await.unwrap();

    let raw = merlin::governance::apps::mint_token();
    let hash = merlin::governance::apps::hash_token(&raw);
    store.put_token("myapp", "tid-1", &hash).await.unwrap();

    let request = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {raw}"))
        .body(axum::body::Body::from(
            serde_json::json!({
                "model": "default",
                "messages": [{"role": "user", "content": "hello"}]
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"]["code"], "budget_exhausted");
}

#[tokio::test]
async fn denials_are_counted_in_metrics() {
    use tower::ServiceExt;
    let (app, _store) = governed_app(AppPolicy::default()).await;

    // Trigger a 401 (missing token).
    let response = app.clone().oneshot(chat_request()).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);

    // The denial is exported on /metrics with its code label.
    let metrics_req = axum::http::Request::builder()
        .method(axum::http::Method::GET)
        .uri("/metrics")
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(metrics_req).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert!(
        text.contains("merlin_governance_denials_total"),
        "metrics body: {text}"
    );
    assert!(
        text.contains("code=\"unauthorized\""),
        "metrics body: {text}"
    );
}

#[tokio::test]
async fn decision_endpoint_applies_governance() {
    use tower::ServiceExt;
    let (app, _store) = governed_app(AppPolicy::default()).await;

    let request = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/v1/decision")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::json!({
                "model": "default",
                "messages": [{"role": "user", "content": "hello"}]
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}
