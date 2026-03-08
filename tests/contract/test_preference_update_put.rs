use merlin::api::endpoints::preferences::*;
use serde_json::{json, Value};
use warp::http::StatusCode;
use warp::test::request;

#[tokio::test]
async fn test_preference_update_put_contract() {
    // Test updating an existing preference
    let request_body = json!({
        "user_id": "user123",
        "preference_key": "preferred_models",
        "preference_value": ["gpt-4", "claude-3", "gemini-pro"],
        "category": "ModelSelection"
    });

    let response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&preference_update_endpoint())
        .await;

    // This test should fail initially because the endpoint doesn't exist yet
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"].is_object());
    assert!(response_body["data"]["id"].is_string());
    assert_eq!(response_body["data"]["user_id"], "user123");
    assert_eq!(response_body["data"]["preference_key"], "preferred_models");
    assert!(response_body["data"]["preference_value"].is_array());
    assert_eq!(response_body["data"]["category"], "ModelSelection");
    assert!(response_body["data"]["created_at"].is_string());
    assert!(response_body["data"]["updated_at"].is_string());
    assert!(response_body["data"]["version"].is_number());
}

#[tokio::test]
async fn test_preference_update_put_new_preference() {
    // Test updating a preference that doesn't exist yet (should create it)
    let request_body = json!({
        "user_id": "user123",
        "preference_key": "response_style",
        "preference_value": "concise",
        "category": "ResponseFormatting"
    });

    let response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert_eq!(response_body["data"]["user_id"], "user123");
    assert_eq!(response_body["data"]["preference_key"], "response_style");
    assert_eq!(response_body["data"]["preference_value"], "concise");
    assert_eq!(response_body["data"]["category"], "ResponseFormatting");
}

#[tokio::test]
async fn test_preference_update_put_validation_error() {
    // Test validation error - missing required fields
    let request_body = json!({
        "user_id": "user123",
        "preference_key": "preferred_models"
        // Missing preference_value (required field)
    });

    let response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(!response_body["success"].as_bool().unwrap());
    assert!(response_body["error"].is_object());
}

#[tokio::test]
async fn test_preference_update_put_category_validation() {
    // Test validation error - invalid category
    let request_body = json!({
        "user_id": "user123",
        "preference_key": "some_key",
        "preference_value": "some_value",
        "category": "InvalidCategory" // Invalid category
    });

    let response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&request_body)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_preference_update_put_unauthorized() {
    // Test unauthorized access
    let request_body = json!({
        "user_id": "user123",
        "preference_key": "preferred_models",
        "preference_value": ["gpt-4"],
        "category": "ModelSelection"
    });

    let response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        // Missing authorization header
        .body_json(&request_body)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}