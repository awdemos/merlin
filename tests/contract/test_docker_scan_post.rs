use merlin::services::docker_config_service::DockerConfigService;
use serde_json::json;
use warp::test::request;

#[tokio::test]
async fn test_post_docker_scan_valid_image() {
    // Test valid Docker image scanning
    let scan_request = serde_json::json!({
        "image_name": "merlin:hardened",
        "scan_types": ["vulnerability", "configuration"],
        "severity_threshold": "CRITICAL",
        "output_format": "json"
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/scan")
        .json(&scan_request)
        .reply(&service.scan_endpoint())
        .await;

    // Should succeed with valid image (will fail until implemented)
    assert_eq!(response.status(), 200, "Docker scan should succeed with valid image");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "success");
    assert_eq!(body["image"], "merlin:hardened");
    assert!(body["scan_id"].as_str().is_some());
    assert!(body["scan_time"].as_str().is_some());
}

#[tokio::test]
async fn test_post_docker_scan_missing_image() {
    // Test scanning non-existent image
    let scan_request = serde_json::json!({
        "image_name": "nonexistent:image",
        "scan_types": ["vulnerability"]
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/scan")
        .json(&scan_request)
        .reply(&service.scan_endpoint())
        .await;

    // Should fail with non-existent image
    assert_eq!(response.status(), 404, "Docker scan should fail with non-existent image");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["message"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_post_docker_scan_invalid_scan_type() {
    // Test invalid scan type
    let scan_request = serde_json::json!({
        "image_name": "merlin:hardened",
        "scan_types": ["invalid_scan_type"],
        "severity_threshold": "CRITICAL"
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/scan")
        .json(&scan_request)
        .reply(&service.scan_endpoint())
        .await;

    // Should fail with invalid scan type
    assert_eq!(response.status(), 400, "Docker scan should fail with invalid scan type");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["message"].as_str().unwrap().contains("scan type"));
}

#[tokio::test]
async fn test_post_docker_scan_critical_vulnerabilities() {
    // Test scanning image with critical vulnerabilities
    let scan_request = serde_json::json!({
        "image_name": "merlin:vulnerable",
        "scan_types": ["vulnerability"],
        "severity_threshold": "CRITICAL"
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/scan")
        .json(&scan_request)
        .reply(&service.scan_endpoint())
        .await;

    // Should return scan results with critical vulnerabilities
    assert_eq!(response.status(), 200, "Docker scan should return results even with vulnerabilities");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "success");
    assert!(body["vulnerabilities"].as_array().is_some());

    // Check if critical vulnerabilities are flagged
    let vulnerabilities = body["vulnerabilities"].as_array().unwrap();
    let has_critical = vulnerabilities.iter().any(|v| v["severity"] == "CRITICAL");
    assert!(has_critical, "Should detect critical vulnerabilities");
}

#[tokio::test]
async fn test_post_docker_scan_configuration_validation() {
    // Test configuration scanning
    let scan_request = serde_json::json!({
        "image_name": "merlin:hardened",
        "scan_types": ["configuration"],
        "checks": ["non_root_user", "read_only_fs", "capability_dropping"]
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/scan")
        .json(&scan_request)
        .reply(&service.scan_endpoint())
        .await;

    // Should return configuration scan results
    assert_eq!(response.status(), 200, "Docker scan should return configuration results");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "success");
    assert!(body["configuration_checks"].as_object().is_some());

    let checks = body["configuration_checks"].as_object().unwrap();
    assert!(checks.contains_key("non_root_user"));
    assert!(checks.contains_key("read_only_fs"));
    assert!(checks.contains_key("capability_dropping"));
}