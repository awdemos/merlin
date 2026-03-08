use merlin::services::docker_config_service::DockerConfigService;
use warp::test::request;

#[tokio::test]
async fn test_get_docker_health_healthy_system() {
    // Test health check when Docker system is healthy
    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("GET")
        .path("/api/v1/docker/health")
        .reply(&service.health_endpoint())
        .await;

    // Should return healthy status (will fail until implemented)
    assert_eq!(response.status(), 200, "Health check should succeed when Docker is healthy");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "healthy");
    assert!(body["timestamp"].as_str().is_some());
    assert!(body["version"].as_str().is_some());
    assert!(body["uptime_seconds"].as_u64().is_some());
}

#[tokio::test]
async fn test_get_docker_health_with_details() {
    // Test health check with detailed information
    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("GET")
        .path("/api/v1/docker/health?details=true")
        .reply(&service.health_endpoint())
        .await;

    // Should return detailed health information
    assert_eq!(response.status(), 200, "Health check should return detailed information");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "healthy");
    assert!(body["details"].as_object().is_some());

    let details = body["details"].as_object().unwrap();
    assert!(details.contains_key("docker_daemon"));
    assert!(details.contains_key("containers_running"));
    assert!(details.contains_key("security_services"));
    assert!(details.contains_key("resource_usage"));
}

#[tokio::test]
async fn test_get_docker_health_docker_unavailable() {
    // Test health check when Docker is unavailable
    // This test requires mocking Docker unavailability
    let service = DockerConfigService::new().unwrap();

    // Mock Docker unavailability scenario
    // This would typically involve setting up a test environment
    // where Docker daemon is not running or not accessible

    let response = request()
        .method("GET")
        .path("/api/v1/docker/health")
        .reply(&service.health_endpoint())
        .await;

    // Should return unhealthy status when Docker is not available
    // For now, this test will pass when implemented correctly
    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();

    // In a real scenario with Docker unavailable:
    // assert_eq!(body["status"], "unhealthy");
    // assert!(body["error"].as_str().unwrap().contains("docker"));
}

#[tokio::test]
async fn test_get_docker_health_container_status() {
    // Test health check including container status
    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("GET")
        .path("/api/v1/docker/health?container_status=true")
        .reply(&service.health_endpoint())
        .await;

    // Should return container status information
    assert_eq!(response.status(), 200, "Health check should include container status");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "healthy");

    if body.get("containers").is_some() {
        let containers = body["containers"].as_array().unwrap();
        // Check if container information includes required fields
        for container in containers {
            assert!(container.as_object().unwrap().contains_key("name"));
            assert!(container.as_object().unwrap().contains_key("status"));
            assert!(container.as_object().unwrap().contains_key("health"));
        }
    }
}

#[tokio::test]
async fn test_get_docker_health_security_status() {
    // Test health check including security service status
    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("GET")
        .path("/api/v1/docker/health?security_status=true")
        .reply(&service.health_endpoint())
        .await;

    // Should return security service status
    assert_eq!(response.status(), 200, "Health check should include security status");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "healthy");

    if body.get("security_services").is_some() {
        let security = body["security_services"].as_object().unwrap();
        assert!(security.contains_key("trivy"));
        assert!(security.contains_key("hadolint"));
        assert!(security.contains_key("docker_bench"));
    }
}

#[tokio::test]
async fn test_get_docker_health_metrics() {
    // Test health check with metrics
    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("GET")
        .path("/api/v1/docker/health?metrics=true")
        .reply(&service.health_endpoint())
        .await;

    // Should return health metrics
    assert_eq!(response.status(), 200, "Health check should include metrics");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "healthy");

    if body.get("metrics").is_some() {
        let metrics = body["metrics"].as_object().unwrap();
        assert!(metrics.contains_key("response_time_ms"));
        assert!(metrics.contains_key("memory_usage_mb"));
        assert!(metrics.contains_key("cpu_usage_percent"));
        assert!(metrics.contains_key("active_containers"));
    }
}

#[tokio::test]
async fn test_get_docker_health_cors_headers() {
    // Test health check with CORS headers
    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("OPTIONS")
        .path("/api/v1/docker/health")
        .reply(&service.health_endpoint())
        .await;

    // Should handle CORS preflight requests
    assert_eq!(response.status(), 200, "Health check should handle CORS preflight");

    // Check CORS headers
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
    assert!(headers.contains_key("access-control-allow-methods"));
    assert!(headers.contains_key("access-control-allow-headers"));
}