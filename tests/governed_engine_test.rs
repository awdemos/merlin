//! Governed engine path: eligibility filtering before bandit selection.

use merlin::config::{AlgorithmConfig, ClientConfig, MerlinConfig, RouteConfig, TargetConfig};
use merlin::engine::RouterEngine;
use merlin::protocol::LlmResponse;

fn test_config(local_url: &str, cloud_url: &str) -> MerlinConfig {
    let mut config = MerlinConfig::default();
    config.server.port = 0;
    config.clients.insert(
        "local".to_string(),
        ClientConfig {
            format: "openai_chat".to_string(),
            base_url: local_url.to_string(),
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
            base_url: cloud_url.to_string(),
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
    config
}

#[tokio::test]
async fn governed_execute_stays_inside_eligible_set() {
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    let local_mock = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                          "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        })))
        .mount(&local_mock)
        .await;

    // The "cloud" client must have a non-loopback base_url for the locality
    // assertions below; eligibility confines this test to local_t, so the
    // cloud endpoint is never actually dialed.
    let engine =
        RouterEngine::new(test_config(&local_mock.uri(), "https://api.openai.com/v1")).unwrap();

    assert!(engine.target_locality("local_t"));
    assert!(!engine.target_locality("cloud_t"));
    assert_eq!(engine.target_price("cloud_t"), Some((10.0, 20.0)));
    assert_eq!(engine.target_price("local_t"), None);

    let mut scope = engine.route_scope("default");
    scope.sort();
    assert_eq!(scope, vec!["cloud_t".to_string(), "local_t".to_string()]);

    let request = merlin::protocol::Request {
        llm_request: merlin::protocol::LlmRequest {
            model: Some("default".to_string()),
            messages: vec![merlin::protocol::Message::text(
                merlin::protocol::Role::User,
                "hello",
            )],
            ..Default::default()
        },
        ..Default::default()
    };

    let response = engine
        .execute_governed("default", request, Some(&["local_t".to_string()]))
        .await
        .expect("governed execute succeeds");
    match response {
        LlmResponse::Aggregate(agg) => {
            assert_eq!(agg.preservation.selected_target.as_deref(), Some("local_t"));
        }
        LlmResponse::Stream(_) => panic!("expected aggregate"),
    }
}

#[tokio::test]
async fn governed_execute_bails_when_algorithm_escapes_eligibility() {
    let mut config = test_config("http://127.0.0.1:1", "http://127.0.0.1:2");
    config.routes.insert(
        "pt".to_string(),
        RouteConfig {
            id: "pt".to_string(),
            default_model: None,
            // Declared targets include an eligible one, so the pre-filtered
            // candidate list is non-empty; the denial must come from the
            // post-selection check on the algorithm's fixed target.
            targets: vec!["cloud_t".to_string(), "local_t".to_string()],
            algorithm: AlgorithmConfig::Passthrough {
                target: "cloud_t".to_string(),
            },
        },
    );
    let engine = RouterEngine::new(config).unwrap();
    let request = merlin::protocol::Request {
        llm_request: merlin::protocol::LlmRequest {
            model: Some("pt".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let err = engine
        .execute_governed("pt", request, Some(&["local_t".to_string()]))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("policy_denied"), "got: {msg}");
    assert!(msg.contains("outside eligible set"), "got: {msg}");
}

#[tokio::test]
async fn governed_execute_denies_when_bandit_selection_escapes_eligibility() {
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    let local_mock = MockServer::start().await;
    let cloud_mock = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                          "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        })))
        .mount(&local_mock)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                          "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        })))
        .mount(&cloud_mock)
        .await;

    let mut config = test_config(&local_mock.uri(), &cloud_mock.uri());
    // Route with cloud first: the epsilon-greedy all-zero-reward
    // tie-break selects the FIRST arm (cloud_t) — outside the eligible
    // set below. The engine must deny, not dispatch.
    config.routes.insert(
        "cloudfirst".to_string(),
        RouteConfig {
            id: "cloudfirst".to_string(),
            default_model: None,
            targets: vec!["cloud_t".to_string(), "local_t".to_string()],
            algorithm: AlgorithmConfig::EpsilonGreedy {
                targets: vec!["cloud_t".to_string(), "local_t".to_string()],
                epsilon: 0.0,
            },
        },
    );
    let engine = RouterEngine::new(config).unwrap();
    let request = merlin::protocol::Request {
        llm_request: merlin::protocol::LlmRequest {
            model: Some("cloudfirst".to_string()),
            messages: vec![merlin::protocol::Message::text(
                merlin::protocol::Role::User,
                "hello",
            )],
            ..Default::default()
        },
        ..Default::default()
    };
    let err = engine
        .execute_governed("cloudfirst", request, Some(&["local_t".to_string()]))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("policy_denied"), "got: {msg}");
    assert!(msg.contains("outside eligible set"), "got: {msg}");
    // The cloud upstream must NOT have been called.
    let cloud_requests = cloud_mock.received_requests().await.unwrap();
    assert!(
        cloud_requests.is_empty(),
        "cloud target was contacted: {cloud_requests:?}"
    );
}

#[tokio::test]
async fn decide_governed_returns_eligible_target() {
    let engine = RouterEngine::new(test_config(
        "http://127.0.0.1:1",
        "https://api.openai.com/v1",
    ))
    .unwrap();
    let request = merlin::protocol::Request {
        llm_request: merlin::protocol::LlmRequest {
            model: Some("default".to_string()),
            messages: vec![merlin::protocol::Message::text(
                merlin::protocol::Role::User,
                "hello",
            )],
            ..Default::default()
        },
        ..Default::default()
    };
    let outcome = engine
        .decide_governed("default", request, Some(&["local_t".to_string()]))
        .await
        .expect("governed decide succeeds");
    assert_eq!(outcome.target.name, "local_t");
}

#[tokio::test]
async fn decide_governed_denies_escape() {
    let mut config = test_config("http://127.0.0.1:1", "https://api.openai.com/v1");
    // Cloud-first arm ordering: the all-zero-reward tie-break picks cloud_t,
    // which is outside the eligible set — decide must deny, not return it.
    config.routes.insert(
        "cloudfirst".to_string(),
        RouteConfig {
            id: "cloudfirst".to_string(),
            default_model: None,
            targets: vec!["cloud_t".to_string(), "local_t".to_string()],
            algorithm: AlgorithmConfig::EpsilonGreedy {
                targets: vec!["cloud_t".to_string(), "local_t".to_string()],
                epsilon: 0.0,
            },
        },
    );
    let engine = RouterEngine::new(config).unwrap();
    let request = merlin::protocol::Request {
        llm_request: merlin::protocol::LlmRequest {
            model: Some("cloudfirst".to_string()),
            messages: vec![merlin::protocol::Message::text(
                merlin::protocol::Role::User,
                "hello",
            )],
            ..Default::default()
        },
        ..Default::default()
    };
    let err = engine
        .decide_governed("cloudfirst", request, Some(&["local_t".to_string()]))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("policy_denied"), "got: {msg}");
    assert!(msg.contains("outside eligible set"), "got: {msg}");
}

#[tokio::test]
async fn governed_execute_denies_empty_scope() {
    let mut config = test_config("http://127.0.0.1:1", "http://127.0.0.1:2");
    config.routes.insert(
        "pt".to_string(),
        RouteConfig {
            id: "pt".to_string(),
            default_model: None,
            targets: vec!["cloud_t".to_string(), "local_t".to_string()],
            algorithm: AlgorithmConfig::Passthrough {
                target: "cloud_t".to_string(),
            },
        },
    );
    let engine = RouterEngine::new(config).unwrap();
    let request = merlin::protocol::Request {
        llm_request: merlin::protocol::LlmRequest {
            model: Some("pt".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let eligible: Vec<String> = Vec::new();
    let err = engine
        .execute_governed("pt", request, Some(&eligible))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("policy_denied"), "got: {msg}");
    assert!(msg.contains("no eligible targets"), "got: {msg}");
}

#[tokio::test]
async fn governed_fallbacks_stay_eligible() {
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    let local_mock = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                          "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        })))
        .mount(&local_mock)
        .await;

    let engine =
        RouterEngine::new(test_config(&local_mock.uri(), "https://api.openai.com/v1")).unwrap();
    let request = merlin::protocol::Request {
        llm_request: merlin::protocol::LlmRequest {
            model: Some("default".to_string()),
            messages: vec![merlin::protocol::Message::text(
                merlin::protocol::Role::User,
                "hello",
            )],
            ..Default::default()
        },
        ..Default::default()
    };
    let response = engine
        .execute_governed("default", request, Some(&["local_t".to_string()]))
        .await
        .expect("governed execute succeeds");
    match response {
        LlmResponse::Aggregate(agg) => {
            assert_eq!(agg.preservation.selected_target.as_deref(), Some("local_t"));
            let fallback_path = agg.preservation.fallback_path.unwrap_or_default();
            assert!(
                !fallback_path.iter().any(|t| t == "cloud_t"),
                "ineligible target in fallback path: {fallback_path:?}"
            );
        }
        LlmResponse::Stream(_) => panic!("expected aggregate"),
    }
}
