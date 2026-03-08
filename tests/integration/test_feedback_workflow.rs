use merlin::api::endpoints::model_selection::*;
use merlin::api::endpoints::feedback::*;
use serde_json::{json, Value};
use warp::http::StatusCode;
use warp::test::request;

#[tokio::test]
async fn test_feedback_submission_workflow() {
    // Step 1: Get a model selection response
    let model_request = json!({
        "user_id": "user123",
        "prompt": "What is the capital of France?",
        "max_tokens": 100,
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

    let model_response_body: Value = serde_json::from_slice(&model_response.body()).unwrap();
    assert!(model_response_body["success"].as_bool().unwrap());

    let request_id = model_response_body["data"]["request_id"].as_str().unwrap();
    let selected_model = model_response_body["data"]["selected_model"].as_str().unwrap();

    // Step 2: Submit feedback for the response
    let feedback_request = json!({
        "user_id": "user123",
        "request_id": request_id,
        "model_name": selected_model,
        "rating": 5,
        "feedback_text": "Perfect answer! Very helpful and accurate.",
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
    assert!(feedback_response_body["data"]["id"].is_string());
    assert_eq!(feedback_response_body["data"]["user_id"], "user123");
    assert_eq!(feedback_response_body["data"]["request_id"], request_id);
    assert_eq!(feedback_response_body["data"]["model_name"], selected_model);
    assert_eq!(feedback_response_body["data"]["rating"], 5);
    assert_eq!(feedback_response_body["data"]["feedback_text"], "Perfect answer! Very helpful and accurate.");
    assert_eq!(feedback_response_body["data"]["category"], "Helpfulness");
}

#[tokio::test]
async fn test_feedback_rating_only_workflow() {
    // Step 1: Get a model selection response
    let model_request = json!({
        "user_id": "user456",
        "prompt": "Explain photosynthesis briefly",
        "max_tokens": 150,
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
    let selected_model = model_response_body["data"]["selected_model"].as_str().unwrap();

    // Step 2: Submit feedback with only rating
    let feedback_request = json!({
        "user_id": "user456",
        "model_name": selected_model,
        "rating": 4,
        "category": "Accuracy"
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
    assert_eq!(feedback_response_body["data"]["user_id"], "user456");
    assert_eq!(feedback_response_body["data"]["model_name"], selected_model);
    assert_eq!(feedback_response_body["data"]["rating"], 4);
    assert_eq!(feedback_response_body["data"]["category"], "Accuracy");
    // Optional fields should be null
    assert!(feedback_response_body["data"]["request_id"].is_null());
    assert!(feedback_response_body["data"]["feedback_text"].is_null());
}

#[tokio::test]
async fn test_feedback_negative_rating_workflow() {
    // Step 1: Get a model selection response
    let model_request = json!({
        "user_id": "user789",
        "prompt": "Write a complex algorithm",
        "max_tokens": 300,
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

    // Step 2: Submit negative feedback
    let feedback_request = json!({
        "user_id": "user789",
        "request_id": request_id,
        "model_name": selected_model,
        "rating": 1,
        "feedback_text": "The response was confusing and didn't address my question properly.",
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
    assert_eq!(feedback_response_body["data"]["rating"], 1);
    assert_eq!(feedback_response_body["data"]["category"], "Helpfulness");
}

#[tokio::test]
async fn test_feedback_multiple_submissions_workflow() {
    let user_id = "user_multi";
    let base_request = json!({
        "user_id": user_id,
        "prompt": "Simple test prompt",
        "max_tokens": 50,
        "temperature": 0.5
    });

    // Submit multiple feedback entries for the same user
    for i in 1..=3 {
        // Get model selection
        let model_response = request()
            .method("POST")
            .path("/api/v1/modelSelect")
            .header("content-type", "application/json")
            .header("authorization", "Bearer test-api-key")
            .body_json(&base_request)
            .reply(&model_select_endpoint())
            .await;

        assert_eq!(model_response.status(), StatusCode::OK);

        let model_response_body: Value = serde_json::from_slice(&model_response.body()).unwrap();
        let selected_model = model_response_body["data"]["selected_model"].as_str().unwrap();

        // Submit feedback
        let feedback_request = json!({
            "user_id": user_id,
            "model_name": selected_model,
            "rating": i % 5 + 1, // Cycle through ratings 1-5
            "category": "Accuracy"
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
        assert_eq!(feedback_response_body["data"]["user_id"], user_id);
        assert_eq!(feedback_response_body["data"]["rating"], i % 5 + 1);
    }
}