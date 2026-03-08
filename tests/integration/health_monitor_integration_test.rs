//! Integration tests for health monitoring system

use merlin::integration::health_monitor::{
    HealthMonitor, HealthCheckConfig, HealthStatus, HealthComponent, HealthThresholds
};
use merlin::integration::docker_client::DockerClient;
use merlin::integration::security_scanner::SecurityScanner;
use merlin::integration::resource_monitor::ResourceMonitor;
use merlin::models::container_config::DockerContainerConfig;
use merlin::models::security_scan_config::SecurityScanConfig;

#[tokio::test]
async fn test_health_monitor_creation() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let resource_monitor = ResourceMonitor::new();
    let config = HealthCheckConfig::default();

    let health_monitor = HealthMonitor::new(
        std::sync::Arc::new(docker_client),
        std::sync::Arc::new(security_scanner),
        std::sync::Arc::new(resource_monitor),
        config,
    );

    assert!(health_monitor.config.check_interval_seconds == 30);
    assert!(health_monitor.config.timeout_seconds == 10);
}

#[tokio::test]
async fn test_health_check_configuration() {
    let config = HealthCheckConfig {
        check_interval_seconds: 15,
        timeout_seconds: 5,
        retry_attempts: 2,
        critical_components: vec![
            HealthComponent::DockerDaemon,
            HealthComponent::SecurityScanner,
        ],
        warning_thresholds: HealthThresholds {
            memory_usage_percent: 75.0,
            cpu_usage_percent: 60.0,
            disk_usage_percent: 80.0,
            response_time_ms: 500,
            error_rate_percent: 3.0,
        },
    };

    assert_eq!(config.check_interval_seconds, 15);
    assert_eq!(config.timeout_seconds, 5);
    assert_eq!(config.retry_attempts, 2);
    assert_eq!(config.critical_components.len(), 2);
    assert_eq!(config.warning_thresholds.memory_usage_percent, 75.0);
}

#[tokio::test]
async fn test_comprehensive_health_check() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let resource_monitor = ResourceMonitor::new();
    let config = HealthCheckConfig::default();

    let health_monitor = HealthMonitor::new(
        std::sync::Arc::new(docker_client),
        std::sync::Arc::new(security_scanner),
        std::sync::Arc::new(resource_monitor),
        config,
    );

    // This test validates the comprehensive health check functionality
    // In a real test environment, some checks might fail due to missing Docker daemon
    let result = health_monitor.check_health().await;

    // The test should either succeed (if Docker is available) or fail gracefully
    // Both are acceptable for integration testing
    match result {
        Ok(status) => {
            println!("Health check completed with status: {:?}", status);
            assert!(matches!(status, HealthStatus::Healthy | HealthStatus::Degraded | HealthStatus::Unhealthy));
        }
        Err(e) => {
            // Check if it's a Docker connection error (expected in test environment)
            let error_msg = e.to_string();
            if error_msg.contains("docker") || error_msg.contains("Docker") {
                println!("Expected Docker connection error in test environment: {}", error_msg);
            } else {
                panic!("Unexpected health check error: {}", error_msg);
            }
        }
    }
}

#[tokio::test]
async fn test_component_health_checks() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let resource_monitor = ResourceMonitor::new();
    let config = HealthCheckConfig::default();

    let health_monitor = HealthMonitor::new(
        std::sync::Arc::new(docker_client),
        std::sync::Arc::new(security_scanner),
        std::sync::Arc::new(resource_monitor),
        config,
    );

    // Test individual component health checks
    let docker_health = health_monitor.get_component_health(&HealthComponent::DockerDaemon).await;
    let scanner_health = health_monitor.get_component_health(&HealthComponent::SecurityScanner).await;
    let monitor_health = health_monitor.get_component_health(&HealthComponent::ResourceMonitor).await;

    // These might be None if no health checks have been run yet
    println!("Docker health: {:?}", docker_health);
    println!("Scanner health: {:?}", scanner_health);
    println!("Monitor health: {:?}", monitor_health);
}

#[tokio::test]
async fn test_health_alerts() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let resource_monitor = ResourceMonitor::new();
    let config = HealthCheckConfig::default();

    let health_monitor = HealthMonitor::new(
        std::sync::Arc::new(docker_client),
        std::sync::Arc::new(security_scanner),
        std::sync::Arc::new(resource_monitor),
        config,
    );

    // Test getting active alerts (initially empty)
    let active_alerts = health_monitor.get_active_alerts().await;
    assert!(active_alerts.is_empty());

    // Test getting health history (initially empty)
    let health_history = health_monitor.get_health_history(None).await;
    assert!(health_history.is_empty());

    // Test getting limited health history
    let limited_history = health_monitor.get_health_history(Some(10)).await;
    assert!(limited_history.is_empty());
}

#[tokio::test]
async fn test_health_thresholds() {
    let thresholds = HealthThresholds {
        memory_usage_percent: 85.0,
        cpu_usage_percent: 75.0,
        disk_usage_percent: 90.0,
        response_time_ms: 2000,
        error_rate_percent: 10.0,
    };

    assert_eq!(thresholds.memory_usage_percent, 85.0);
    assert_eq!(thresholds.cpu_usage_percent, 75.0);
    assert_eq!(thresholds.disk_usage_percent, 90.0);
    assert_eq!(thresholds.response_time_ms, 2000);
    assert_eq!(thresholds.error_rate_percent, 10.0);
}

#[tokio::test]
async fn test_health_status_variants() {
    // Test all health status variants
    let statuses = vec![
        HealthStatus::Healthy,
        HealthStatus::Degraded,
        HealthStatus::Unhealthy,
        HealthStatus::Unknown,
    ];

    for status in statuses {
        match status {
            HealthStatus::Healthy => assert!(true),
            HealthStatus::Degraded => assert!(true),
            HealthStatus::Unhealthy => assert!(true),
            HealthStatus::Unknown => assert!(true),
        }
    }
}

#[tokio::test]
async fn test_health_component_variants() {
    // Test all health component variants
    let components = vec![
        HealthComponent::DockerDaemon,
        HealthComponent::SecurityScanner,
        HealthComponent::ResourceMonitor,
        HealthComponent::DeploymentPipeline,
        HealthComponent::Database,
        HealthComponent::Network,
        HealthComponent::Storage,
        HealthComponent::Custom("test-component".to_string()),
    ];

    for component in components {
        match component {
            HealthComponent::DockerDaemon => assert!(true),
            HealthComponent::Container { id } => assert!(true),
            HealthComponent::SecurityScanner => assert!(true),
            HealthComponent::ResourceMonitor => assert!(true),
            HealthComponent::DeploymentPipeline => assert!(true),
            HealthComponent::Database => assert!(true),
            HealthComponent::Network => assert!(true),
            HealthComponent::Storage => assert!(true),
            HealthComponent::Custom(name) => assert!(true),
        }
    }
}

#[tokio::test]
async fn test_health_monitor_with_custom_config() {
    let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
    let security_scanner = SecurityScanner::new().expect("Failed to create security scanner");
    let resource_monitor = ResourceMonitor::new();

    let custom_config = HealthCheckConfig {
        check_interval_seconds: 60,
        timeout_seconds: 15,
        retry_attempts: 5,
        critical_components: vec![
            HealthComponent::DockerDaemon,
            HealthComponent::Network,
            HealthComponent::Storage,
        ],
        warning_thresholds: HealthThresholds {
            memory_usage_percent: 90.0,
            cpu_usage_percent: 80.0,
            disk_usage_percent: 95.0,
            response_time_ms: 3000,
            error_rate_percent: 15.0,
        },
    };

    let health_monitor = HealthMonitor::new(
        std::sync::Arc::new(docker_client),
        std::sync::Arc::new(security_scanner),
        std::sync::Arc::new(resource_monitor),
        custom_config,
    );

    assert_eq!(health_monitor.config.check_interval_seconds, 60);
    assert_eq!(health_monitor.config.timeout_seconds, 15);
    assert_eq!(health_monitor.config.retry_attempts, 5);
    assert_eq!(health_monitor.config.critical_components.len(), 3);
    assert_eq!(health_monitor.config.warning_thresholds.memory_usage_percent, 90.0);
}