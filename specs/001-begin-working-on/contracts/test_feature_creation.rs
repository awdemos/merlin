// Contract tests for feature creation
// These tests MUST fail before implementation

use serde_json::{json, Value};

#[cfg(test)]
mod feature_creation_tests {
    use super::*;

    // Test: Create feature with valid data
    #[tokio::test]
    async fn test_create_feature_valid_data() {
        // This test will fail until implementation is complete
        let client = reqwest::Client::new();

        let response = client
            .post("http://localhost:8080/api/v1/features")
            .json(&json!({
                "name": "test-feature",
                "description": "Test feature for contract validation",
                "metadata": {
                    "priority": "Medium",
                    "tags": ["test"]
                }
            }))
            .send()
            .await
            .unwrap();

        // Should succeed with 201 Created
        assert_eq!(response.status(), 201);

        let feature: Value = response.json().await.unwrap();
        assert_eq!(feature["name"], "test-feature");
        assert_eq!(feature["number"], 1);
        assert!(feature["id"].as_str().unwrap().starts_with("001-"));
    }

    // Test: Create feature with duplicate name should fail
    #[tokio::test]
    async fn test_create_feature_duplicate_name() {
        let client = reqwest::Client::new();

        // First request should succeed
        let response1 = client
            .post("http://localhost:8080/api/v1/features")
            .json(&json!({
                "name": "duplicate-feature",
                "description": "First instance"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response1.status(), 201);

        // Second request with same name should fail
        let response2 = client
            .post("http://localhost:8080/api/v1/features")
            .json(&json!({
                "name": "duplicate-feature",
                "description": "Second instance"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response2.status(), 409);
    }

    // Test: Create feature with invalid name should fail
    #[tokio::test]
    async fn test_create_feature_invalid_name() {
        let client = reqwest::Client::new();

        let response = client
            .post("http://localhost:8080/api/v1/features")
            .json(&json!({
                "name": "Invalid Name With Spaces",
                "description": "Test feature"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 400);
    }

    // Test: Create feature with empty description should fail
    #[tokio::test]
    async fn test_create_feature_empty_description() {
        let client = reqwest::Client::new();

        let response = client
            .post("http://localhost:8080/api/v1/features")
            .json(&json!({
                "name": "test-feature",
                "description": ""
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 400);
    }

    // Test: Feature should receive sequential numbers
    #[tokio::test]
    async fn test_sequential_feature_numbers() {
        let client = reqwest::Client::new();

        // Create first feature
        let response1 = client
            .post("http://localhost:8080/api/v1/features")
            .json(&json!({
                "name": "first-feature",
                "description": "First feature"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response1.status(), 201);
        let feature1: Value = response1.json().await.unwrap();
        assert_eq!(feature1["number"], 1);

        // Create second feature
        let response2 = client
            .post("http://localhost:8080/api/v1/features")
            .json(&json!({
                "name": "second-feature",
                "description": "Second feature"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response2.status(), 201);
        let feature2: Value = response2.json().await.unwrap();
        assert_eq!(feature2["number"], 2);
    }

    // Test: Created feature should have required fields
    #[tokio::test]
    async fn test_created_feature_structure() {
        let client = reqwest::Client::new();

        let response = client
            .post("http://localhost:8080/api/v1/features")
            .json(&json!({
                "name": "structure-test",
                "description": "Test feature structure"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 201);
        let feature: Value = response.json().await.unwrap();

        // Verify required fields
        assert!(feature["id"].is_string());
        assert!(feature["number"].is_number());
        assert!(feature["name"].is_string());
        assert!(feature["description"].is_string());
        assert!(feature["status"].is_string());
        assert!(feature["created_at"].is_string());
        assert!(feature["updated_at"].is_string());
        assert!(feature["branch_name"].is_string());

        // Verify default values
        assert_eq!(feature["status"], "Draft");
        assert_eq!(feature["metadata"]["priority"], "Medium");
    }
}