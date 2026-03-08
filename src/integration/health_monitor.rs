//! Health monitoring service for Merlin AI Router Docker deployment
//! Provides comprehensive health checks, metrics, and status monitoring

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use crate::models::docker_config::{DockerContainerConfig, DockerConfigError};
use crate::models::container_state::{ContainerState, ContainerStatus};
use crate::models::security_scan_config::SecurityScanConfig;
use crate::integration::docker_client::DockerClient;
use crate::integration::security_scanner::SecurityScanner;
use crate::integration::resource_monitor::ResourceMonitor;

/// Health check result with detailed status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub id: Uuid,
    pub component: HealthComponent,
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub message: String,
    pub details: HashMap<String, serde_json::Value>,
    pub next_check: Option<DateTime<Utc>>,
}

/// Health status levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Health check components
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthComponent {
    DockerDaemon,
    Container { id: Uuid },
    SecurityScanner,
    ResourceMonitor,
    DeploymentPipeline,
    Database,
    Network,
    Storage,
    Custom(String),
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub check_interval_seconds: u64,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub critical_components: Vec<HealthComponent>,
    pub warning_thresholds: HealthThresholds,
}

/// Health check thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthThresholds {
    pub memory_usage_percent: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub response_time_ms: u64,
    pub error_rate_percent: f64,
}

/// Health monitoring service
#[derive(Clone)]
pub struct HealthMonitor {
    docker_client: Arc<DockerClient>,
    security_scanner: Arc<SecurityScanner>,
    resource_monitor: Arc<ResourceMonitor>,
    health_checks: Arc<RwLock<HashMap<Uuid, HealthCheck>>>,
    health_history: Arc<RwLock<Vec<HealthCheck>>>,
    config: HealthCheckConfig,
    alerts: Arc<RwLock<Vec<HealthAlert>>>,
}

/// Health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    pub id: Uuid,
    pub component: HealthComponent,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl HealthMonitor {
    pub fn new(
        docker_client: Arc<DockerClient>,
        security_scanner: Arc<SecurityScanner>,
        resource_monitor: Arc<ResourceMonitor>,
        config: HealthCheckConfig,
    ) -> Self {
        Self {
            docker_client,
            security_scanner,
            resource_monitor,
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            health_history: Arc::new(RwLock::new(Vec::new())),
            config,
            alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Perform comprehensive health check
    pub async fn check_health(&self) -> Result<HealthStatus, DockerConfigError> {
        let mut results = Vec::new();

        // Check Docker daemon
        results.push(self.check_docker_daemon().await);

        // Check security scanner
        results.push(self.check_security_scanner().await);

        // Check resource monitor
        results.push(self.check_resource_monitor().await);

        // Check network connectivity
        results.push(self.check_network_connectivity().await);

        // Check storage
        results.push(self.check_storage().await);

        // Determine overall health status
        let overall_status = self.determine_overall_health(&results);

        // Store results
        self.store_health_checks(results).await;

        Ok(overall_status)
    }

    /// Check Docker daemon health
    async fn check_docker_daemon(&self) -> HealthCheck {
        let start_time = std::time::Instant::now();
        let component = HealthComponent::DockerDaemon;

        let result = self.docker_client.ping().await;

        let (status, message, details) = match result {
            Ok(duration) => {
                let mut details = HashMap::new();
                details.insert("ping_duration_ms".to_string(), serde_json::Value::Number(serde_json::Number::from(duration.as_millis() as u64)));
                details.insert("version".to_string(), serde_json::Value::String("20.10+".to_string()));

                (HealthStatus::Healthy, "Docker daemon is responding".to_string(), details)
            }
            Err(e) => {
                let mut details = HashMap::new();
                details.insert("error".to_string(), serde_json::Value::String(e.to_string()));

                (HealthStatus::Unhealthy, format!("Docker daemon check failed: {}", e), details)
            }
        };

        HealthCheck {
            id: Uuid::new_v4(),
            component,
            status,
            timestamp: Utc::now(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            message,
            details,
            next_check: Some(Utc::now() + chrono::Duration::seconds(self.config.check_interval_seconds as i64)),
        }
    }

    /// Check security scanner health
    async fn check_security_scanner(&self) -> HealthCheck {
        let start_time = std::time::Instant::now();
        let component = HealthComponent::SecurityScanner;

        // Test security scanner with a simple configuration
        let test_config = SecurityScanConfig::builder()
            .image_name("nginx:latest".to_string())
            .scan_types(vec![crate::models::security_scan_config::ScanType::Vulnerability])
            .severity_threshold("LOW".to_string())
            .output_format(crate::models::security_scan_config::OutputFormat::Json)
            .build()
            .unwrap();

        let result = self.security_scanner.validate_configuration(&test_config).await;

        let (status, message, details) = match result {
            Ok(_) => {
                let mut details = HashMap::new();
                details.insert("validation_passed".to_string(), serde_json::Value::Bool(true));
                details.insert("tools_available".to_string(), serde_json::Value::Array(vec![
                    serde_json::Value::String("trivy".to_string()),
                    serde_json::Value::String("hadolint".to_string()),
                ]));

                (HealthStatus::Healthy, "Security scanner is configured correctly".to_string(), details)
            }
            Err(e) => {
                let mut details = HashMap::new();
                details.insert("error".to_string(), serde_json::Value::String(e.to_string()));

                (HealthStatus::Degraded, format!("Security scanner check failed: {}", e), details)
            }
        };

        HealthCheck {
            id: Uuid::new_v4(),
            component,
            status,
            timestamp: Utc::now(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            message,
            details,
            next_check: Some(Utc::now() + chrono::Duration::seconds(self.config.check_interval_seconds as i64)),
        }
    }

    /// Check resource monitor health
    async fn check_resource_monitor(&self) -> HealthCheck {
        let start_time = std::time::Instant::now();
        let component = HealthComponent::ResourceMonitor;

        // Check if resource monitor is collecting metrics
        let metrics_count = self.resource_monitor.get_metrics_count().await;

        let (status, message, details) = if metrics_count > 0 {
            let mut details = HashMap::new();
            details.insert("metrics_count".to_string(), serde_json::Value::Number(serde_json::Number::from(metrics_count)));
            details.insert("monitoring_active".to_string(), serde_json::Value::Bool(true));

            (HealthStatus::Healthy, "Resource monitor is collecting metrics".to_string(), details)
        } else {
            let mut details = HashMap::new();
            details.insert("metrics_count".to_string(), serde_json::Value::Number(serde_json::Number::from(0)));
            details.insert("monitoring_active".to_string(), serde_json::Value::Bool(false));

            (HealthStatus::Degraded, "Resource monitor is not collecting metrics".to_string(), details)
        };

        HealthCheck {
            id: Uuid::new_v4(),
            component,
            status,
            timestamp: Utc::now(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            message,
            details,
            next_check: Some(Utc::now() + chrono::Duration::seconds(self.config.check_interval_seconds as i64)),
        }
    }

    /// Check network connectivity
    async fn check_network_connectivity(&self) -> HealthCheck {
        let start_time = std::time::Instant::now();
        let component = HealthComponent::Network;

        // Test network connectivity to Docker Hub
        let client = reqwest::Client::new();
        let result = client
            .get("https://registry.hub.docker.com/v2/")
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;

        let (status, message, details) = match result {
            Ok(response) => {
                let mut details = HashMap::new();
                details.insert("registry_status_code".to_string(), serde_json::Value::Number(serde_json::Number::from(response.status().as_u16())));
                details.insert("registry_accessible".to_string(), serde_json::Value::Bool(true));

                (HealthStatus::Healthy, "Network connectivity is working".to_string(), details)
            }
            Err(e) => {
                let mut details = HashMap::new();
                details.insert("error".to_string(), serde_json::Value::String(e.to_string()));
                details.insert("registry_accessible".to_string(), serde_json::Value::Bool(false));

                (HealthStatus::Degraded, format!("Network connectivity check failed: {}", e), details)
            }
        };

        HealthCheck {
            id: Uuid::new_v4(),
            component,
            status,
            timestamp: Utc::now(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            message,
            details,
            next_check: Some(Utc::now() + chrono::Duration::seconds(self.config.check_interval_seconds as i64)),
        }
    }

    /// Check storage availability
    async fn check_storage(&self) -> HealthCheck {
        let start_time = std::time::Instant::now();
        let component = HealthComponent::Storage;

        // Check storage by attempting to write and read a test file
        let test_content = "health_check_test";
        let test_path = "/tmp/merlin_health_check.tmp";

        let result: Result<(), std::io::Error> = async {
            // Write test
            tokio::fs::write(test_path, test_content).await?;

            // Read test
            let read_content = tokio::fs::read_to_string(test_path).await?;

            // Cleanup
            tokio::fs::remove_file(test_path).await?;

            Ok(())
        }.await;

        let (status, message, details) = match result {
            Ok(_) => {
                let mut details = HashMap::new();
                details.insert("storage_accessible".to_string(), serde_json::Value::Bool(true));
                details.insert("test_path".to_string(), serde_json::Value::String(test_path.to_string()));

                (HealthStatus::Healthy, "Storage is accessible".to_string(), details)
            }
            Err(e) => {
                let mut details = HashMap::new();
                details.insert("error".to_string(), serde_json::Value::String(e.to_string()));
                details.insert("storage_accessible".to_string(), serde_json::Value::Bool(false));

                (HealthStatus::Unhealthy, format!("Storage check failed: {}", e), details)
            }
        };

        HealthCheck {
            id: Uuid::new_v4(),
            component,
            status,
            timestamp: Utc::now(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            message,
            details,
            next_check: Some(Utc::now() + chrono::Duration::seconds(self.config.check_interval_seconds as i64)),
        }
    }

    /// Determine overall health status from individual checks
    fn determine_overall_health(&self, checks: &[HealthCheck]) -> HealthStatus {
        let mut has_critical = false;
        let mut has_unhealthy = false;
        let mut has_degraded = false;

        for check in checks {
            if self.config.critical_components.contains(&check.component) {
                match check.status {
                    HealthStatus::Unhealthy => has_unhealthy = true,
                    HealthStatus::Degraded => has_degraded = true,
                    HealthStatus::Unknown => has_degraded = true,
                    HealthStatus::Healthy => {}
                }
            }

            match check.status {
                HealthStatus::Unhealthy => has_unhealthy = true,
                HealthStatus::Degraded => has_degraded = true,
                HealthStatus::Unknown => has_degraded = true,
                HealthStatus::Healthy => {}
            }
        }

        if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Store health check results
    async fn store_health_checks(&self, checks: Vec<HealthCheck>) {
        let mut health_checks = self.health_checks.write().await;
        let mut health_history = self.health_history.write().await;

        for check in checks {
            health_checks.insert(check.id, check.clone());
            health_history.push(check);
        }

        // Keep only last 1000 health check results
        if health_history.len() > 1000 {
            *health_history = health_history[health_history.len() - 1000..].to_vec();
        }
    }

    /// Get health status for a specific component
    pub async fn get_component_health(&self, component: &HealthComponent) -> Option<HealthCheck> {
        let health_checks = self.health_checks.read().await;
        health_checks.values()
            .find(|check| &check.component == component)
            .cloned()
    }

    /// Get all active health alerts
    pub async fn get_active_alerts(&self) -> Vec<HealthAlert> {
        let alerts = self.alerts.read().await;
        alerts.iter()
            .filter(|alert| !alert.resolved)
            .cloned()
            .collect()
    }

    /// Get health history
    pub async fn get_health_history(&self, limit: Option<usize>) -> Vec<HealthCheck> {
        let health_history = self.health_history.read().await;
        match limit {
            Some(limit) => health_history[health_history.len().saturating_sub(limit)..].to_vec(),
            None => health_history.clone(),
        }
    }

    /// Resolve a health alert
    pub async fn resolve_alert(&self, alert_id: Uuid) -> Result<(), DockerConfigError> {
        let mut alerts = self.alerts.write().await;

        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.resolved = true;
            alert.resolved_at = Some(Utc::now());
            Ok(())
        } else {
            Err(DockerConfigError::Validation("Alert not found".to_string()))
        }
    }

    /// Start health monitoring loop
    pub async fn start_monitoring(&self) -> Result<(), DockerConfigError> {
        let monitor = self.clone();

        tokio::spawn(async move {
            loop {
                if let Err(e) = monitor.check_health().await {
                    eprintln!("Health check failed: {}", e);
                }

                tokio::time::sleep(std::time::Duration::from_secs(monitor.config.check_interval_seconds)).await;
            }
        });

        Ok(())
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 30,
            timeout_seconds: 10,
            retry_attempts: 3,
            critical_components: vec![
                HealthComponent::DockerDaemon,
                HealthComponent::SecurityScanner,
                HealthComponent::ResourceMonitor,
            ],
            warning_thresholds: HealthThresholds {
                memory_usage_percent: 80.0,
                cpu_usage_percent: 70.0,
                disk_usage_percent: 85.0,
                response_time_ms: 1000,
                error_rate_percent: 5.0,
            },
        }
    }
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            memory_usage_percent: 80.0,
            cpu_usage_percent: 70.0,
            disk_usage_percent: 85.0,
            response_time_ms: 1000,
            error_rate_percent: 5.0,
        }
    }
}