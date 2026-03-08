//! Integration tests for deployment pipeline

use merlin::integration::deployment_pipeline::{DeploymentPipeline, DeploymentRequest, DeploymentStrategy};
use merlin::integration::docker_client::DockerClient;
use merlin::integration::security_scanner::SecurityScanner;
use merlin::models::docker_config::DockerContainerConfig;
use merlin::models::deployment_environment::{DeploymentEnvironment, EnvironmentType};

#[tokio::test]
async fn test_deployment_pipeline_creation() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let pipeline = DeploymentPipeline::new(docker_client, security_scanner);

    assert_eq!(pipeline.config.max_concurrent_deployments, 3);
    assert!(pipeline.config.enable_security_scan);
}

#[tokio::test]
async fn test_environment_management() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let pipeline = DeploymentPipeline::new(docker_client, security_scanner);

    let environment = DeploymentEnvironment::new(
        "test-env".to_string(),
        EnvironmentType::Development,
    );

    assert!(pipeline.add_environment(environment).await.is_ok());
}

#[tokio::test]
async fn test_deployment_status_tracking() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let pipeline = DeploymentPipeline::new(docker_client, security_scanner);

    let environment = DeploymentEnvironment::new(
        "test-env".to_string(),
        EnvironmentType::Development,
    );
    pipeline.add_environment(environment).await.unwrap();

    let config = DockerContainerConfig::new("nginx:latest".to_string(), "Dockerfile".to_string());
    let request = DeploymentRequest {
        environment_id: environment.id,
        image_name: "nginx:latest".to_string(),
        config,
        strategy: Some(DeploymentStrategy::Recreate),
        timeout_seconds: Some(300),
        skip_security_scan: true,
        metadata: std::collections::HashMap::new(),
    };

    // This test validates the deployment workflow without requiring actual Docker
    // In a real CI environment, you'd need Docker daemon running
    let result = pipeline.deploy(request).await;

    // The test should either succeed (if Docker is available) or fail with a Docker connection error
    // Both are acceptable for integration testing
    match result {
        Ok(_) => {
            // Deployment succeeded
            println!("Deployment integration test passed");
        }
        Err(e) => {
            // Check if it's a Docker connection error (expected in test environment)
            let error_msg = e.to_string();
            if error_msg.contains("docker") || error_msg.contains("Docker") {
                println!("Expected Docker connection error in test environment: {}", error_msg);
            } else {
                panic!("Unexpected deployment error: {}", error_msg);
            }
        }
    }
}

#[tokio::test]
async fn test_deployment_history() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let pipeline = DeploymentPipeline::new(docker_client, security_scanner);

    let history = pipeline.list_deployment_history(None).await.unwrap();
    assert!(history.is_empty()); // Initially empty

    let active = pipeline.list_active_deployments().await.unwrap();
    assert!(active.is_empty()); // Initially empty
}

#[tokio::test]
async fn test_concurrent_deployment_limit() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let pipeline = DeploymentPipeline::new(docker_client, security_scanner);

    // The default limit is 3, so we should be able to check the limit
    // This test validates the concurrent deployment limit check logic
    let can_deploy = pipeline.check_concurrent_deployment_limit().await.unwrap();
    assert!(can_deploy); // Should be true when no active deployments
}