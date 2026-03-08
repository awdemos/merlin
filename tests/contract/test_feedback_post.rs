use merlin::api::endpoints::feedback::*;
use serde_json::{json, Value};
use warp::http::StatusCode;
use warp::test::request;

#[tokio::test]
async fn test_feedback_post_contract_rating_only() {
    // Test feedback submission with rating only
    let request_body = json!({
        "user_id": "user123",
        "model_name": "gpt-4",
        "rating": 5,
        "category": "Accuracy"
    });

    let response = request()
        .method("POST")
        .path("/api/v1/feedback")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&feedback_endpoint())
        .await;

    // This test should fail initially because the endpoint doesn't exist yet
    assert_eq!(response.status(), StatusCode::CREATED);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"].is_object());
    assert!(response_body["data"]["id"].is_string());
    assert_eq!(response_body["data"]["user_id"], "user123");
    assert_eq!(response_body["data"]["model_name"], "gpt-4");
    assert_eq!(response_body["data"]["rating"], 5);
    assert_eq!(response_body["data"]["category"], "Accuracy");
    assert!(response_body["data"]["created_at"].is_string());
}

#[tokio::test]
async fn test_feedback_post_contract_with_details() {
    // Test feedback submission with detailed feedback
    let request_body = json!({
        "user_id": "user123",
        "request_id": "123e4567-e89b-12d3-a456-426614174000",
        "model_name": "claude-3",
        "rating": 4,
        "feedback_text": "The response was helpful but could be more concise",
        "category": "Helpfulness"
    });

    let response = request()
        .method("POST")
        .path("/api/v1/feedback")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&feedback_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert_eq!(response_body["data"]["user_id"], "user123");
    assert_eq!(response_body["data"]["request_id"], "123e4567-e89b-12d3-a456-426614174000");
    assert_eq!(response_body["data"]["model_name"], "claude-3");
    assert_eq!(response_body["data"]["rating"], 4);
    assert_eq!(response_body["data"]["feedback_text"], "The response was helpful but could be more concise");
    assert_eq!(response_body["data"]["category"], "Helpfulness");
}

#[tokio::test]
async fn test_feedback_post_validation_error() {
    // Test validation error - missing required fields
    let request_body = json!({
        "user_id": "user123",
        "rating": 5
        // Missing model_name (required field)
    });

    let response = request()
        .method("POST")
        .path("/api/v1/feedback")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&feedback_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(!response_body["success"].as_bool().unwrap());
    assert!(response_body["error"].is_object());
}

#[tokio::test]
async fn test_feedback_post_rating_validation() {
    // Test validation error - rating out of range
    let request_body = json!({
        "user_id": "user123",
        "model_name": "gpt-4",
        "rating": 6, // Invalid rating (should be 1-5)
        "category": "Accuracy"
    });

    let response = request()
        .method("POST")
        .path("/api/v1/feedback")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&feedback_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_feedback_post_category_validation() {
    // Test validation error - invalid category
    let request_body = json!({
        "user_id": "user123",
        "model_name": "gpt-4",
        "rating": 5,
        "category": "InvalidCategory" // Invalid category
    });

    let response = request()
        .method("POST")
        .path("/api/v1/feedback")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&feedback_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}