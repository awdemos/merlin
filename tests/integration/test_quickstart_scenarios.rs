use merlin::api::endpoints::model_selection::*;
use merlin::api::endpoints::feedback::*;
use merlin::api::endpoints::preferences::*;
use serde_json::{json, Value};
use warp::http::StatusCode;
use warp::test::request;

#[tokio::test]
async fn test_quickstart_scenario_1_basic_model_selection() {
    // Scenario: User wants to select the best model for a simple question
    let request_body = json!({
        "user_id": "quickstart_user_1",
        "prompt": "What is the weather like today?",
        "max_tokens": 100,
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

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"]["selected_model"].is_string());
    assert!(response_body["data"]["response"].is_string());
    assert!(response_body["data"]["tokens_used"].is_number());
    assert!(response_body["data"]["processing_time"].is_number());
    assert!(response_body["data"]["confidence_score"].is_number());
}

#[tokio::test]
async fn test_quickstart_scenario_2_model_selection_with_preferences() {
    // Scenario: User with model preferences wants to get a response
    let user_id = "quickstart_user_2";

    // First, set up user preferences
    let preference_request = json!({
        "user_id": user_id,
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

    // Now make model selection request
    let model_request = json!({
        "user_id": user_id,
        "prompt": "Explain the concept of machine learning",
        "max_tokens": 300,
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

    assert_eq!(model_response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&model_response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());

    let selected_model = response_body["data"]["selected_model"].as_str().unwrap();
    let preferred_models = ["gpt-4", "claude-3"];
    assert!(preferred_models.contains(&selected_model));
}

#[tokio::test]
async fn test_quickstart_scenario_3_feedback_submission() {
    // Scenario: User submits feedback after receiving a response
    let user_id = "quickstart_user_3";

    // Get model selection
    let model_request = json!({
        "user_id": user_id,
        "prompt": "Write a short poem about technology",
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

    let model_response_body: Value = serde_json::from_slice(&model_response.body()).unwrap();
    let request_id = model_response_body["data"]["request_id"].as_str().unwrap();
    let selected_model = model_response_body["data"]["selected_model"].as_str().unwrap();

    // Submit feedback
    let feedback_request = json!({
        "user_id": user_id,
        "request_id": request_id,
        "model_name": selected_model,
        "rating": 4,
        "feedback_text": "Creative and well-written poem!",
        "category": "Helpfulness"
    });

    let feedback_response = request()
        .method("POST")
        .path("/api/v1/feedback")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&feedback_request)
        .reply(&feedback_endpoint())
        .await;

    assert_eq!(feedback_response.status(), StatusCode::CREATED);

    let feedback_response_body: Value = serde_json::from_slice(&feedback_response.body()).unwrap();
    assert!(feedback_response_body["success"].as_bool().unwrap());
    assert_eq!(feedback_response_body["data"]["request_id"], request_id);
    assert_eq!(feedback_response_body["data"]["model_name"], selected_model);
}

#[tokio::test]
async fn test_quickstart_scenario_4_preference_management() {
    // Scenario: User manages their preferences
    let user_id = "quickstart_user_4";
    let preference_key = "response_style";

    // Create preference
    let create_request = json!({
        "user_id": user_id,
        "preference_key": preference_key,
        "preference_value": "detailed",
        "category": "ResponseFormatting"
    });

    let create_response = request()
        .method("POST")
        .path("/api/v1/preferences/userPreferenceCreate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&create_request)
        .reply(&preference_create_endpoint())
        .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Update preference
    let update_request = json!({
        "user_id": user_id,
        "preference_key": preference_key,
        "preference_value": "concise",
        "category": "ResponseFormatting"
    });

    let update_response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&update_request)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(update_response.status(), StatusCode::OK);

    let update_body: Value = serde_json::from_slice(&update_response.body()).unwrap();
    assert_eq!(update_body["data"]["preference_value"], "concise");

    // Delete preference
    let delete_response = request()
        .method("DELETE")
        .path(&format!("/api/v1/preferences/userPreferenceDelete?user_id={}&preference_key={}", user_id, preference_key))
        .header("authorization", "Bearer test-api-key")
        .reply(&preference_delete_endpoint())
        .await;

    assert_eq!(delete_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_quickstart_scenario_5_complex_workflow() {
    // Scenario: Complete workflow from preference setup to feedback
    let user_id = "quickstart_user_5";

    // Setup multiple preferences
    let preferences = vec![
        ("preferred_models", json!(["gpt-4", "claude-3"]), "ModelSelection"),
        ("response_style", json!("detailed"), "ResponseFormatting"),
        ("max_tokens", json!(500), "ModelSelection"),
    ];

    for (key, value, category) in preferences {
        let request = json!({
            "user_id": user_id,
            "preference_key": key,
            "preference_value": value,
            "category": category
        });

        let response = request()
            .method("POST")
            .path("/api/v1/preferences/userPreferenceCreate")
            .header("content-type", "application/json")
            .header("authorization", "Bearer test-api-key")
            .body_json(&request)
            .reply(&preference_create_endpoint())
            .await;

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    // Make model selection with preferences
    let model_request = json!({
        "user_id": user_id,
        "prompt": "Explain quantum computing and its potential applications",
        "max_tokens": 400,
        "temperature": 0.6
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

    let model_response_body: Value = serde_json::from_slice(&model_response.body()).unwrap();
    let request_id = model_response_body["data"]["request_id"].as_str().unwrap();
    let selected_model = model_response_body["data"]["selected_model"].as_str().unwrap();

    // Verify model is from preferred list
    let preferred_models = ["gpt-4", "claude-3"];
    assert!(preferred_models.contains(&selected_model));

    // Submit comprehensive feedback
    let feedback_request = json!({
        "user_id": user_id,
        "request_id": request_id,
        "model_name": selected_model,
        "rating": 5,
        "feedback_text": "Excellent explanation! Very detailed and easy to understand.",
        "category": "Helpfulness"
    });

    let feedback_response = request()
        .method("POST")
        .path("/api/v1/feedback")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&feedback_request)
        .reply(&feedback_endpoint())
        .await;

    assert_eq!(feedback_response.status(), StatusCode::CREATED);

    let feedback_response_body: Value = serde_json::from_slice(&feedback_response.body()).unwrap();
    assert!(feedback_response_body["success"].as_bool().unwrap());
    assert_eq!(feedback_response_body["data"]["rating"], 5);
}

#[tokio::test]
async fn test_quickstart_scenario_6_error_handling() {
    // Scenario: Test error handling for invalid requests
    let user_id = "quickstart_user_6";

    // Test invalid model selection (missing required field)
    let invalid_request = json!({
        "prompt": "Test prompt",
        "max_tokens": 100
        // Missing user_id
    });

    let response = request()
        .method("POST")
        .path("/api/v1/modelSelect")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&invalid_request)
        .reply(&model_select_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Test invalid preference creation (invalid category)
    let invalid_preference = json!({
        "user_id": user_id,
        "preference_key": "test_key",
        "preference_value": "test_value",
        "category": "InvalidCategory"
    });

    let pref_response = request()
        .method("POST")
        .path("/api/v1/preferences/userPreferenceCreate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&invalid_preference)
        .reply(&preference_create_endpoint())
        .await;

    assert_eq!(pref_response.status(), StatusCode::BAD_REQUEST);

    // Test invalid feedback (invalid rating)
    let invalid_feedback = json!({
        "user_id": user_id,
        "model_name": "gpt-4",
        "rating": 6, // Invalid rating (should be 1-5)
        "category": "Accuracy"
    });

    let feedback_response = request()
        .method("POST")
        .path("/api/v1/feedback")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&invalid_feedback)
        .reply(&feedback_endpoint())
        .await;

    assert_eq!(feedback_response.status(), StatusCode::BAD_REQUEST);
}