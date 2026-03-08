use merlin::api::endpoints::preferences::*;
use serde_json::{json, Value};
use warp::http::StatusCode;
use warp::test::request;

#[tokio::test]
async fn test_preference_delete_delete_contract() {
    // Test deleting an existing preference
    let response = request()
        .method("DELETE")
        .path("/api/v1/preferences/userPreferenceDelete?user_id=user123&preference_key=preferred_models")
        .header("authorization", "Bearer test-api-key")
        .reply(&preference_delete_endpoint())
        .await;

    // This test should fail initially because the endpoint doesn't exist yet
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"].is_object());
    assert!(response_body["data"]["deleted"].as_bool().unwrap());
    assert_eq!(response_body["data"]["user_id"], "user123");
    assert_eq!(response_body["data"]["preference_key"], "preferred_models");
}

#[tokio::test]
async fn test_preference_delete_delete_nonexistent() {
    // Test deleting a preference that doesn't exist
    let response = request()
        .method("DELETE")
        .path("/api/v1/preferences/userPreferenceDelete?user_id=user123&preference_key=nonexistent_preference")
        .header("authorization", "Bearer test-api-key")
        .reply(&preference_delete_endpoint())
        .await;

    // Should still return OK (idempotent operation)
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(response_body["success"].as_bool().unwrap());
    assert!(response_body["data"]["deleted"].as_bool().unwrap());
    assert_eq!(response_body["data"]["user_id"], "user123");
    assert_eq!(response_body["data"]["preference_key"], "nonexistent_preference");
}

#[tokio::test]
async fn test_preference_delete_delete_missing_user_id() {
    // Test validation error - missing user_id parameter
    let response = request()
        .method("DELETE")
        .path("/api/v1/preferences/userPreferenceDelete?preference_key=preferred_models")
        .header("authorization", "Bearer test-api-key")
        .reply(&preference_delete_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(!response_body["success"].as_bool().unwrap());
    assert!(response_body["error"].is_object());
}

#[tokio::test]
async fn test_preference_delete_delete_missing_preference_key() {
    // Test validation error - missing preference_key parameter
    let response = request()
        .method("DELETE")
        .path("/api/v1/preferences/userPreferenceDelete?user_id=user123")
        .header("authorization", "Bearer test-api-key")
        .reply(&preference_delete_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response_body: Value = serde_json::from_slice(&response.body()).unwrap();
    assert!(!response_body["success"].as_bool().unwrap());
    assert!(response_body["error"].is_object());
}

#[tokio::test]
async fn test_preference_delete_delete_unauthorized() {
    // Test unauthorized access
    let response = request()
        .method("DELETE")
        .path("/api/v1/preferences/userPreferenceDelete?user_id=user123&preference_key=preferred_models")
        // Missing authorization header
        .reply(&preference_delete_endpoint())
        .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}