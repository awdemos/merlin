//! End-to-end tests using wiremock upstreams.

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use merlin::config::MerlinConfig;
    use merlin::engine::RouterEngine;
    use merlin::protocol::LlmResponse;
    use std::sync::Arc;
    use tower::ServiceExt;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    async fn build_engine_with_mock(
        mock: &MockServer,
        format: &str,
    ) -> anyhow::Result<RouterEngine> {
        let mut config = MerlinConfig::default();
        config.server.port = 0;

        let client_name = match format {
            "anthropic_messages" => "anthropic",
            _ => "openai",
        };
        config.clients.insert(
            client_name.to_string(),
            merlin::config::ClientConfig {
                format: format.to_string(),
                base_url: mock.uri(),
                api_key_env: None,
                max_retries: 0,
                forward_auth: false,
                extra_headers: Default::default(),
                timeout_ms: 5000,
            },
        );
        config.targets.insert(
            "mock_target".to_string(),
            merlin::config::TargetConfig {
                name: "mock_target".to_string(),
                client: client_name.to_string(),
                model: "mock-model".to_string(),
                extra_body: None,
            },
        );
        config.targets.insert(
            "fallback_target".to_string(),
            merlin::config::TargetConfig {
                name: "fallback_target".to_string(),
                client: client_name.to_string(),
                model: "fallback-model".to_string(),
                extra_body: None,
            },
        );
        config.routes.insert(
            "default".to_string(),
            merlin::config::RouteConfig {
                id: "default".to_string(),
                default_model: None,
                targets: vec!["mock_target".to_string()],
                algorithm: merlin::config::AlgorithmConfig::Passthrough {
                    target: "mock_target".to_string(),
                },
            },
        );
        Ok(RouterEngine::new(config)?)
    }

    #[tokio::test]
    async fn openai_passthrough_returns_completion() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "model": "mock-model",
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": "Hello from mock" },
                    "finish_reason": "stop"
                }],
                "usage": { "prompt_tokens": 10, "completion_tokens": 4, "total_tokens": 14 }
            })))
            .mount(&mock)
            .await;

        let engine = build_engine_with_mock(&mock, "openai_chat").await.unwrap();
        let request = serde_json::json!({
            "model": "default",
            "messages": [{"role": "user", "content": "hi"}],
        });
        let app = merlin::server::create_app(Arc::new(engine));
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            body["choices"][0]["message"]["content"].as_str().unwrap(),
            "Hello from mock"
        );
        assert_eq!(body["model"].as_str().unwrap(), "mock-model");
    }

    #[tokio::test]
    async fn engine_execute_openai_non_streaming() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "model": "mock-model",
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": "Engine works" },
                    "finish_reason": "stop"
                }],
                "usage": { "prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7 }
            })))
            .mount(&mock)
            .await;

        let engine = build_engine_with_mock(&mock, "openai_chat").await.unwrap();
        let req = merlin::translation::openai_decode::decode_openai_request(serde_json::json!({
            "model": "default",
            "messages": [{"role": "user", "content": "hello"}],
        }))
        .unwrap();
        let response = engine.execute("default", req).await.unwrap();
        match response {
            LlmResponse::Aggregate(agg) => {
                assert_eq!(agg.assistant_text(), "Engine works");
                assert_eq!(agg.model, "mock-model");
            }
            _ => panic!("expected aggregate response"),
        }
    }

    #[tokio::test]
    async fn fallback_activates_on_upstream_failure() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&mock)
            .await;
        let fallback = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "fb",
                "object": "chat.completion",
                "model": "fallback-model",
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": "Fallback OK" },
                    "finish_reason": "stop"
                }],
                "usage": { "prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2 }
            })))
            .mount(&fallback)
            .await;

        let mut config = MerlinConfig::default();
        config.clients.insert(
            "openai".to_string(),
            merlin::config::ClientConfig {
                format: "openai_chat".to_string(),
                base_url: mock.uri(),
                api_key_env: None,
                max_retries: 0,
                forward_auth: false,
                extra_headers: Default::default(),
                timeout_ms: 1000,
            },
        );
        config.targets.insert(
            "primary".to_string(),
            merlin::config::TargetConfig {
                name: "primary".to_string(),
                client: "openai".to_string(),
                model: "primary-model".to_string(),
                extra_body: None,
            },
        );
        config.targets.insert(
            "fallback".to_string(),
            merlin::config::TargetConfig {
                name: "fallback".to_string(),
                client: "openai".to_string(),
                model: "fallback-model".to_string(),
                extra_body: None,
            },
        );
        // Fallback target uses different client pointing to fallback server.
        config.clients.insert(
            "fallback_client".to_string(),
            merlin::config::ClientConfig {
                format: "openai_chat".to_string(),
                base_url: fallback.uri(),
                api_key_env: None,
                max_retries: 0,
                forward_auth: false,
                extra_headers: Default::default(),
                timeout_ms: 1000,
            },
        );
        config.targets.get_mut("fallback").unwrap().client = "fallback_client".to_string();
        config.routes.insert(
            "default".to_string(),
            merlin::config::RouteConfig {
                id: "default".to_string(),
                default_model: None,
                targets: vec!["primary".to_string(), "fallback".to_string()],
                algorithm: merlin::config::AlgorithmConfig::Passthrough {
                    target: "primary".to_string(),
                },
            },
        );

        let engine = RouterEngine::new(config).unwrap();
        let req = merlin::translation::openai_decode::decode_openai_request(serde_json::json!({
            "model": "default",
            "messages": [{"role": "user", "content": "hi"}],
        }))
        .unwrap();
        let response = engine.execute("default", req).await.unwrap();
        match response {
            LlmResponse::Aggregate(agg) => {
                assert_eq!(agg.assistant_text(), "Fallback OK");
                assert_eq!(agg.model, "fallback-model");
                assert_eq!(
                    agg.preservation.fallback_path,
                    Some(vec!["fallback".to_string()])
                );
            }
            _ => panic!("expected aggregate response"),
        }
    }
}
