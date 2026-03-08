use merlin::api::endpoints::model_selection::*;
use merlin::api::endpoints::preferences::*;
use serde_json::{json, Value};
use warp::http::StatusCode;
use warp::test::request;

#[tokio::test]
async fn test_model_selection_workflow_with_preferences() {
    // Step 1: Create user preferences
    let preference_request = json!({
        "user_id": "user123",
        "preference_key": "preferred_models",
        "preference_value": ["gpt-4", "claude-3"],
        "category": "ModelSelection"
    });

    let pref_response = request()
        .method("POST")
        .path("/api/v1/preferences/userPreferenceCreate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&preference_request)
        .reply(&preference_create_endpoint())
        .await;

    assert_eq!(pref_response.status(), StatusCode::CREATED);

    // Step 2: Make model selection request
    let model_request = json!({
        "user_id": "user123",
        "prompt": "Explain quantum computing in simple terms",
        "max_tokens": 500,
        "temperature": 0.7
    });

    let model_response = request()
        .method("POST")
        .path("/api/v1/modelSelect")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&model_request)
        .reply(&model_select_endpoint())
        .await;

    assert_eq!(model_response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&model_response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"]["selected_model"].is_string());

    // Verify the selected model is one of the preferred models
    let selected_model = response_body["data"]["selected_model"].as_str().unwrap();
    assert!(["gpt-4", "claude-3"].contains(&selected_model));
}

#[tokio::test]
async fn test_model_selection_workflow_with_cost_preferences() {
    // Step 1: Create user preferences with cost constraints
    let preference_request = json!({
        "user_id": "user456",
        "preference_key": "cost_settings",
        "preference_value": {"max_cost": 0.01, "preferred_models": ["gpt-3.5"]},
        "category": "ModelSelection"
    });

    let pref_response = request()
        .method("POST")
        .path("/api/v1/preferences/userPreferenceCreate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&preference_request)
        .reply(&preference_create_endpoint())
        .await;

    assert_eq!(pref_response.status(), StatusCode::CREATED);

    // Step 2: Make model selection request with cost constraints
    let model_request = json!({
        "user_id": "user456",
        "prompt": "What is 2+2?",
        "max_tokens": 100,
        "temperature": 0.5,
        "model_preferences": {
            "max_cost": 0.01
        }
    });

    let model_response = request()
        .method("POST")
        .path("/api/v1/modelSelect")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&model_request)
        .reply(&model_select_endpoint())
        .await;

    assert_eq!(model_response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&model_response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"]["selected_model"].is_string());
    assert!(response_body["data"]["confidence_score"].is_number());
}

#[tokio::test]
async fn test_model_selection_workflow_with_excluded_models() {
    // Step 1: Create user preferences with excluded models
    let preference_request = json!({
        "user_id": "user789",
        "preference_key": "excluded_models",
        "preference_value": ["gpt-3.5"],
        "category": "ModelSelection"
    });

    let pref_response = request()
        .method("POST")
        .path("/api/v1/preferences/userPreferenceCreate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&preference_request)
        .reply(&preference_create_endpoint())
        .await;

    assert_eq!(pref_response.status(), StatusCode::CREATED);

    // Step 2: Make model selection request
    let model_request = json!({
        "user_id": "user789",
        "prompt": "Write a haiku about programming",
        "max_tokens": 200,
        "temperature": 0.8
    });

    let model_response = request()
        .method("POST")
        .path("/api/v1/modelSelect")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&model_request)
        .reply(&model_select_endpoint())
        .await;

    assert_eq!(model_response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&model_response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    let selected_model = response_body["data"]["selected_model"].as_str().unwrap();

    // Verify the selected model is not the excluded one
    assert_ne!(selected_model, "gpt-3.5");
}

#[tokio::test]
async fn test_model_selection_workflow_error_handling() {
    // Test with invalid user ID (no preferences exist)
    let model_request = json!({
        "user_id": "invalid_user",
        "prompt": "Test prompt",
        "max_tokens": 50,
        "temperature": 0.5
    });

    let model_response = request()
        .method("POST")
        .path("/api/v1/modelSelect")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&model_request)
        .reply(&model_select_endpoint())
        .await;

    // Should still work with default behavior
    assert_eq!(model_response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&model_response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"]["selected_model"].is_string());
}