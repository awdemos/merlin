use merlin::services::docker_config_service::DockerConfigService;
use merlin::models::docker_config::DockerContainerConfig;
use serde_json::json;
use warp::test::request;

#[tokio::test]
async fn test_post_docker_build_valid_config() {
    // Test valid Docker build configuration
    let config = DockerContainerConfig {
        image_name: "merlin:hardened".to_string(),
        dockerfile_path: "docker/Dockerfile.hardened".to_string(),
        build_args: vec![("RUST_ENV".to_string(), "production".to_string())],
        tags: vec!["latest".to_string(), "hardened".to_string()],
        security_context: Some(serde_json::json!({
            "user": "1000:1000",
            "read_only": true
        })),
    };

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/build")
        .json(&config)
        .reply(&service.build_endpoint())
        .await;

    // Should succeed with valid config (will fail until implemented)
    assert_eq!(response.status(), 200, "Docker build should succeed with valid configuration");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["image_id"], "merlin:hardened");
    assert!(body["build_time"].as_str().is_some());
}

#[tokio::test]
async fn test_post_docker_build_invalid_config() {
    // Test invalid Docker build configuration
    let invalid_config = serde_json::json!({
        "image_name": "",  // Empty image name should fail
        "dockerfile_path": "nonexistent/Dockerfile"
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/build")
        .json(&invalid_config)
        .reply(&service.build_endpoint())
        .await;

    // Should fail with invalid config
    assert_eq!(response.status(), 400, "Docker build should fail with invalid configuration");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["message"].as_str().is_some());
}

#[tokio::test]
async fn test_post_docker_build_security_validation() {
    // Test security validation during build
    let insecure_config = DockerContainerConfig {
        image_name: "merlin:test".to_string(),
        dockerfile_path: "docker/Dockerfile.insecure".to_string(),
        build_args: vec![],
        tags: vec!["test".to_string()],
        security_context: Some(serde_json::json!({
            "user": "root",  // Running as root should fail
            "read_only": false
        })),
    };

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/build")
        .json(&insecure_config)
        .reply(&service.build_endpoint())
        .await;

    // Should fail due to security violations
    assert_eq!(response.status(), 400, "Docker build should fail with security violations");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["message"].as_str().unwrap().contains("security"));
    assert!(body["violations"].as_array().is_some());
}

#[tokio::test]
async fn test_post_docker_build_missing_required_fields() {
    // Test missing required fields
    let incomplete_config = serde_json::json!({
        "image_name": "merlin:test"
        // Missing dockerfile_path
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/build")
        .json(&incomplete_config)
        .reply(&service.build_endpoint())
        .await;

    // Should fail with missing required fields
    assert_eq!(response.status(), 400, "Docker build should fail with missing required fields");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["message"].as_str().unwrap().contains("required"));
}