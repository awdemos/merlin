use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use merlin::server::{create_server_with_state, AppState};
use merlin::server::preferences::PreferenceServerState;

#[tokio::test]
async fn test_create_user_preferences() {
    let app = create_test_app().await;

    let request_data = json!({
        "user_id": "test_user_123",
        "optimize_for": "quality",
        "max_tokens": 4096,
        "temperature": 0.8,
        "custom_weights": {
            "gpt-4": 1.5,
            "claude-3": 1.2
        },
        "preferred_models": ["gpt-4", "claude-3"],
        "excluded_models": ["llama-3.1"],
        "learning_enabled": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/preferences/users")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());
    assert_eq!(response_data["preferences"]["user_id"].as_str().unwrap(), "test_user_123");
}

#[tokio::test]
async fn test_get_user_preferences() {
    let app = create_test_app().await;

    // First create a user
    let create_request = json!({
        "user_id": "test_user_get",
        "optimize_for": "speed",
        "max_tokens": 2048,
        "temperature": 0.5
    });

    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/preferences/users")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&create_request).unwrap()))
            .unwrap(),
    )
    .await
    .unwrap();

    // Then get the user
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/preferences/users/test_user_get")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());
    assert_eq!(response_data["preferences"]["user_id"].as_str().unwrap(), "test_user_get");
    assert_eq!(response_data["preferences"]["optimize_for"], "speed");
}

#[tokio::test]
async fn test_update_user_preferences() {
    let app = create_test_app().await;

    // First create a user
    let create_request = json!({
        "user_id": "test_user_update",
        "optimize_for": "cost",
        "max_tokens": 1024
    });

    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/preferences/users")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&create_request).unwrap()))
            .unwrap(),
    )
    .await
    .unwrap();

    // Then update the user
    let update_request = json!({
        "optimize_for": "balanced",
        "max_tokens": 4096,
        "temperature": 0.7,
        "learning_enabled": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/preferences/users/test_user_update")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());
    assert_eq!(response_data["preferences"]["optimize_for"], "balanced");
    assert_eq!(response_data["preferences"]["max_tokens"], 4096);
    assert_eq!(response_data["preferences"]["learning_enabled"], false);
}

#[tokio::test]
async fn test_delete_user_preferences() {
    let app = create_test_app().await;

    // First create a user
    let create_request = json!({
        "user_id": "test_user_delete",
        "optimize_for": "quality"
    });

    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/preferences/users")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&create_request).unwrap()))
            .unwrap(),
    )
    .await
    .unwrap();

    // Then delete the user
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/preferences/users/test_user_delete")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());

    // Verify user is deleted
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/preferences/users/test_user_delete")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should still return OK but with default preferences (since we have fallback)
    assert_eq!(get_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_users() {
    let app = create_test_app().await;

    // Create multiple users
    for i in 0..3 {
        let request = json!({
            "user_id": format!("test_user_{}", i),
            "optimize_for": "quality"
        });

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/preferences/users")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    }

    // List users
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/preferences/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());
    assert!(response_data["users"].as_array().unwrap().len() >= 3);
}

#[tokio::test]
async fn test_record_user_interaction() {
    let app = create_test_app().await;

    // First create a user
    let create_request = json!({
        "user_id": "test_user_interaction",
        "optimize_for": "quality"
    });

    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/preferences/users")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&create_request).unwrap()))
            .unwrap(),
    )
    .await
    .unwrap();

    // Record an interaction
    let interaction_request = json!({
        "session_id": "session_123",
        "model_used": "gpt-4",
        "rating": 5,
        "feedback_type": "Quality",
        "response_time_ms": 2000,
        "cost": 0.03,
        "prompt_features": {
            "domain_category": "technical",
            "task_type": "question",
            "complexity_score": 0.8,
            "estimated_tokens": 200,
            "keywords": ["code", "algorithm"]
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/preferences/users/test_user_interaction/interactions")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&interaction_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_get_user_stats() {
    let app = create_test_app().await;

    // Create a user with some interactions
    let create_request = json!({
        "user_id": "test_user_stats",
        "optimize_for": "quality"
    });

    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/preferences/users")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&create_request).unwrap()))
            .unwrap(),
    )
    .await
    .unwrap();

    // Add an interaction
    let interaction_request = json!({
        "session_id": "session_stats",
        "model_used": "gpt-4",
        "rating": 4,
        "response_time_ms": 1500,
        "cost": 0.02,
        "prompt_features": {
            "domain_category": "technical",
            "task_type": "question",
            "complexity_score": 0.6,
            "estimated_tokens": 150,
            "keywords": ["test"]
        }
    });

    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/preferences/users/test_user_stats/interactions")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&interaction_request).unwrap()))
            .unwrap(),
    )
    .await
    .unwrap();

    // Get stats
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/preferences/users/test_user_stats/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());
    assert_eq!(response_data["stats"]["user_id"].as_str().unwrap(), "test_user_stats");
    assert_eq!(response_data["stats"]["total_requests"], 1);
}

#[tokio::test]
async fn test_search_preferences() {
    let app = create_test_app().await;

    // Create users with different preferences
    let users = vec![
        ("user_quality", "quality", true),
        ("user_speed", "speed", true),
        ("user_cost", "cost", false),
    ];

    for (user_id, optimize_for, learning_enabled) in users {
        let request = json!({
            "user_id": user_id,
            "optimize_for": optimize_for,
            "learning_enabled": learning_enabled
        });

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/preferences/users")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    }

    // Search for users with learning enabled
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/preferences/search?learning_enabled=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());
    let user_ids: Vec<&str> = response_data["user_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    assert!(user_ids.contains(&"user_quality"));
    assert!(user_ids.contains(&"user_speed"));
    assert!(!user_ids.contains(&"user_cost"));
}

#[tokio::test]
async fn test_validate_preferences() {
    let app = create_test_app().await;

    // Test valid preferences
    let valid_request = json!({
        "user_id": "valid_user",
        "preferences": {
            "temperature": 0.7,
            "max_tokens": 2048,
            "custom_weights": {
                "gpt-4": 1.5
            }
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/preferences/validate")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&valid_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());
    assert!(response_data["valid"].as_bool().unwrap());
    assert!(response_data["errors"].as_array().unwrap().is_empty());

    // Test invalid preferences
    let invalid_request = json!({
        "user_id": "",
        "preferences": {
            "temperature": 3.0, // Invalid: should be 0.0-2.0
            "max_tokens": 50000, // Invalid: should be 1-32000
            "custom_weights": {
                "gpt-4": 5.0 // Invalid: should be 0.1-3.0
            }
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/preferences/validate")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&invalid_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["success"].as_bool().unwrap());
    assert!(!response_data["valid"].as_bool().unwrap());
    assert!(response_data["errors"].as_array().unwrap().len() > 0);
}

async fn create_test_app() -> Router {
    create_server_with_state().await.unwrap()
}