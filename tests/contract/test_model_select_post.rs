use merlin::api::endpoints::model_selection::*;
use serde_json::{json, Value};
use warp::http::StatusCode;
use warp::test::request;

#[tokio::test]
async fn test_model_select_post_contract() {
    // Test basic model selection request
    let request_body = json!({
        "user_id": "user123",
        "prompt": "Explain quantum computing in simple terms",
        "max_tokens": 500,
        "temperature": 0.7
    });

    let response = request()
        .method("POST")
        .path("/api/v1/modelSelect")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&model_select_endpoint())
        .await;

    // This test should fail initially because the endpoint doesn't exist yet
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"].is_object());
    assert!(response_body["data"]["request_id"].is_string());
    assert!(response_body["data"]["selected_model"].is_string());
    assert!(response_body["data"]["response"].is_string());
    assert!(response_body["data"]["tokens_used"].is_number());
    assert!(response_body["data"]["processing_time"].is_number());
    assert!(response_body["data"]["confidence_score"].is_number());
}

#[tokio::test]
async fn test_model_select_post_with_preferences() {
    // Test advanced model selection with preferences
    let request_body = json!({
        "user_id": "user123",
        "prompt": "Write a Python function to calculate fibonacci numbers",
        "max_tokens": 300,
        "temperature": 0.5,
        "model_preferences": {
            "preferred_models": ["gpt-4", "claude-3"],
            "excluded_models": ["gpt-3.5"],
            "max_cost": 0.02
        },
        "context": {
            "session_id": "session_456",
            "source_application": "web_app"
        }
    });

    let response = request()
        .method("POST")
        .path("/api/v1/modelSelect")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&model_select_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"]["selected_model"].is_string());
}

#[tokio::test]
async fn test_model_select_post_validation_error() {
    // Test validation error - missing required fields
    let request_body = json!({
        "prompt": "Explain quantum computing"
        // Missing user_id (required field)
    });

    let response = request()
        .method("POST")
        .path("/api/v1/modelSelect")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&model_select_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(!response_body["success"].as_bool().unwrap());
    assert!(response_body["error"].is_object());
}

#[tokio::test]
async fn test_model_select_post_unauthorized() {
    // Test unauthorized access
    let request_body = json!({
        "user_id": "user123",
        "prompt": "Explain quantum computing"
    });

    let response = request()
        .method("POST")
        .path("/api/v1/modelSelect")
        .header("content-type", "application/json")
        // Missing authorization header
        .body_json(&request_body)
        .reply(&model_select_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}