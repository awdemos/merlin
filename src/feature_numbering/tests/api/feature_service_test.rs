use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::util::ServiceExt; // for `oneshot` method
use crate::feature_numbering::{api::FeatureApiState, create_router};

// These tests should fail initially since the API doesn't exist yet
#[tokio::test]
async fn test_create_feature_endpoint() {
    let app = create_test_app();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/features")
        .header("content-type", "application/json")
        .body(Body::from(r#"
            {
                "name": "test-feature",
                "description": "Test feature creation via API"
            }
        "#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let feature: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(feature["name"], "test-feature");
    assert_eq!(feature["status"], "Draft");
    assert_eq!(feature["number"], 1);
}

#[tokio::test]
async fn test_get_feature_endpoint() {
    let app = create_test_app();

    // First create a feature
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/v1/features")
        .header("content-type", "application/json")
        .body(Body::from(r#"
            {
                "name": "get-test-feature",
                "description": "Test feature retrieval"
            }
        "#))
        .unwrap();

    let create_response = app.oneshot(create_request).await.unwrap();
    let create_body = hyper::body::to_bytes(create_response.into_body()).await.unwrap();
    let created_feature: Value = serde_json::from_slice(&create_body).unwrap();

    // Then retrieve it
    let get_request = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/features/{}", created_feature["id"].as_str().unwrap()))
        .body(Body::empty())
        .unwrap();

    let get_response = app.oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);

    let get_body = hyper::body::to_bytes(get_response.into_body()).await.unwrap();
    let retrieved_feature: Value = serde_json::from_slice(&get_body).unwrap();
    assert_eq!(retrieved_feature["id"], created_feature["id"]);
}

#[tokio::test]
async fn test_list_features_endpoint() {
    let app = create_test_app();

    // Create a few features first
    for i in 1..=3 {
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/features")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "name": format!("list-test-feature-{}", i),
                "description": format!("Test feature {} for listing", i)
            }).to_string()))
            .unwrap();

        app.oneshot(request).await.unwrap();
    }

    // List all features
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/features")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["features"].is_array());
    assert!(response_data["features"].as_array().unwrap().len() >= 3);
}

#[tokio::test]
async fn test_update_feature_endpoint() {
    let app = create_test_app();

    // Create a feature first
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/v1/features")
        .header("content-type", "application/json")
        .body(Body::from(r#"
            {
                "name": "update-test-feature",
                "description": "Original description"
            }
        "#))
        .unwrap();

    let create_response = app.oneshot(create_request).await.unwrap();
    let create_body = hyper::body::to_bytes(create_response.into_body()).await.unwrap();
    let created_feature: Value = serde_json::from_slice(&create_body).unwrap();

    // Update the feature
    let update_request = Request::builder()
        .method("PUT")
        .uri(&format!("/api/v1/features/{}", created_feature["id"].as_str().unwrap()))
        .header("content-type", "application/json")
        .body(Body::from(r#"
            {
                "description": "Updated description",
                "metadata": {
                    "priority": "High",
                    "tags": ["important", "urgent"]
                }
            }
        "#))
        .unwrap();

    let update_response = app.oneshot(update_request).await.unwrap();
    assert_eq!(update_response.status(), StatusCode::OK);

    let update_body = hyper::body::to_bytes(update_response.into_body()).await.unwrap();
    let updated_feature: Value = serde_json::from_slice(&update_body).unwrap();
    assert_eq!(updated_feature["description"], "Updated description");
    assert_eq!(updated_feature["metadata"]["priority"], "High");
}

#[tokio::test]
async fn test_update_feature_status_endpoint() {
    let app = create_test_app();

    // Create a feature first
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/v1/features")
        .header("content-type", "application/json")
        .body(Body::from(r#"
            {
                "name": "status-test-feature",
                "description": "Test status updates"
            }
        "#))
        .unwrap();

    let create_response = app.oneshot(create_request).await.unwrap();
    let create_body = hyper::body::to_bytes(create_response.into_body()).await.unwrap();
    let created_feature: Value = serde_json::from_slice(&create_body).unwrap();

    // Update status from Draft to Planned
    let status_request = Request::builder()
        .method("PATCH")
        .uri(&format!("/api/v1/features/{}/status", created_feature["id"].as_str().unwrap()))
        .header("content-type", "application/json")
        .body(Body::from(r#"
            {
                "status": "Planned"
            }
        "#))
        .unwrap();

    let status_response = app.oneshot(status_request).await.unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);

    let status_body = hyper::body::to_bytes(status_response.into_body()).await.unwrap();
    let updated_feature: Value = serde_json::from_slice(&status_body).unwrap();
    assert_eq!(updated_feature["status"], "Planned");
}

#[tokio::test]
async fn test_delete_feature_endpoint() {
    let app = create_test_app();

    // Create a feature first
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/v1/features")
        .header("content-type", "application/json")
        .body(Body::from(r#"
            {
                "name": "delete-test-feature",
                "description": "Test feature deletion"
            }
        "#))
        .unwrap();

    let create_response = app.oneshot(create_request).await.unwrap();
    let create_body = hyper::body::to_bytes(create_response.into_body()).await.unwrap();
    let created_feature: Value = serde_json::from_slice(&create_body).unwrap();

    // Delete the feature
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(&format!("/api/v1/features/{}", created_feature["id"].as_str().unwrap()))
        .body(Body::empty())
        .unwrap();

    let delete_response = app.oneshot(delete_request).await.unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // Verify it's deleted
    let get_request = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/features/{}", created_feature["id"].as_str().unwrap()))
        .body(Body::empty())
        .unwrap();

    let get_response = app.oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_next_number_endpoint() {
    let app = create_test_app();

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/numbers/next")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["next_number"].is_number());
    assert!(response_data["next_number"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_reserve_number_endpoint() {
    let app = create_test_app();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/numbers/reserved")
        .header("content-type", "application/json")
        .body(Body::from(r#"
            {
                "number": 42,
                "reason": "Special milestone feature"
            }
        "#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Verify the number is reserved
    let get_request = Request::builder()
        .method("GET")
        .uri("/api/v1/numbers/reserved")
        .body(Body::empty())
        .unwrap();

    let get_response = app.oneshot(get_request).await.unwrap();
    let get_body = hyper::body::to_bytes(get_response.into_body()).await.unwrap();
    let reserved_data: Value = serde_json::from_slice(&get_body).unwrap();

    assert!(reserved_data["reserved_numbers"].is_array());
    let reserved_numbers = reserved_data["reserved_numbers"].as_array().unwrap();
    let found = reserved_numbers.iter().any(|num| num["number"] == 42);
    assert!(found);
}

#[tokio::test]
async fn test_search_features_endpoint() {
    let app = create_test_app();

    // Create features with searchable content
    let features_to_create = vec![
        ("search-auth", "Authentication system implementation"),
        ("search-db", "Database migration and setup"),
        ("search-api", "REST API endpoint development"),
    ];

    for (name, description) in features_to_create {
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/features")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "name": name,
                "description": description
            }).to_string()))
            .unwrap();

        app.oneshot(request).await.unwrap();
    }

    // Search for "authentication"
    let search_request = Request::builder()
        .method("GET")
        .uri("/api/v1/search?q=authentication&field=description")
        .body(Body::empty())
        .unwrap();

    let search_response = app.oneshot(search_request).await.unwrap();
    assert_eq!(search_response.status(), StatusCode::OK);

    let search_body = hyper::body::to_bytes(search_response.into_body()).await.unwrap();
    let search_results: Value = serde_json::from_slice(&search_body).unwrap();

    assert!(search_results["results"].is_array());
    let results = search_results["results"].as_array().unwrap();
    assert!(results.len() > 0);

    // Should find the authentication feature
    let found_auth = results.iter().any(|feature|
        feature["description"].as_str().unwrap().contains("authentication")
    );
    assert!(found_auth);
}

// Helper function to create test app
fn create_test_app() -> Router {
    let state = FeatureApiState::new();
    create_router().with_state(state)
}