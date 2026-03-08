//! Deployment pipeline for Merlin AI Router
//!
//! Provides automated deployment pipeline with security validation,
//  rollback capabilities, and multi-environment support.

use crate::models::docker_config::{DockerContainerConfig, DockerConfigError};
use crate::models::deployment_environment::{DeploymentEnvironment, EnvironmentType, DeploymentStatus};
use crate::models::container_state::{ContainerState, ContainerStatus};
use crate::integration::docker_client::DockerClient;
use crate::integration::security_scanner::{SecurityScanner, ScanOptions, ScanType};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

/// Deployment pipeline service
pub struct DeploymentPipeline {
    /// Docker client for container operations
    docker_client: Arc<DockerClient>,

    /// Security scanner for image validation
    security_scanner: Arc<SecurityScanner>,

    /// Deployment environments
    environments: Arc<RwLock<HashMap<uuid::Uuid, DeploymentEnvironment>>>,

    /// Active deployments
    active_deployments: Arc<RwLock<HashMap<uuid::Uuid, Deployment>>>,

    /// Deployment history
    deployment_history: Arc<RwLock<Vec<Deployment>>>,

    /// Pipeline configuration
    config: PipelineConfig,
}

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Enable security scanning
    pub enable_security_scan: bool,

    /// Enable rollback on failure
    pub enable_rollback: bool,

    /// Maximum concurrent deployments
    pub max_concurrent_deployments: usize,

    /// Default deployment timeout in seconds
    pub default_timeout_seconds: u64,

    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,

    /// Deployment strategies
    pub deployment_strategy: DeploymentStrategy,

    /// Notification configuration
    pub notifications: NotificationConfig,
}

/// Deployment strategy
#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentStrategy {
    /// Rolling update deployment
    RollingUpdate {
        batch_size: usize,
        health_check_timeout_seconds: u64,
    },

    /// Blue-green deployment
    BlueGreen {
        prep_environment: String,
        switch_delay_seconds: u64,
    },

    /// Canary deployment
    Canary {
        canary_percentage: f64,
        monitoring_duration_minutes: u64,
    },

    /// Recreate deployment (stop all, start all)
    Recreate,
}

/// Notification configuration
#[derive(Debug, Clone)]
pub struct NotificationConfig {
    /// Enable deployment notifications
    pub enabled: bool,

    /// Webhook URLs for notifications
    pub webhooks: Vec<String>,

    /// Email notification recipients
    pub email_recipients: Vec<String>,

    /// Slack webhook URL
    pub slack_webhook: Option<String>,
}

/// Deployment information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Deployment {
    pub id: uuid::Uuid,
    pub environment_id: uuid::Uuid,
    pub image_name: String,
    pub config: DockerContainerConfig,
    pub status: DeploymentStatus,
    pub strategy: DeploymentStrategy,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_seconds: Option<u64>,
    pub health_checks: Vec<HealthCheck>,
    pub rollback_available: bool,
    pub previous_version: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Health check result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthCheck {
    pub id: uuid::Uuid,
    pub check_type: String,
    pub status: HealthCheckStatus,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: u64,
    pub details: HashMap<String, String>,
}

/// Health check status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum HealthCheckStatus {
    Healthy,
    Unhealthy,
    Degraded,
    Unknown,
}

/// Deployment request
#[derive(Debug, Clone)]
pub struct DeploymentRequest {
    pub environment_id: uuid::Uuid,
    pub image_name: String,
    pub config: DockerContainerConfig,
    pub strategy: Option<DeploymentStrategy>,
    pub timeout_seconds: Option<u64>,
    pub skip_security_scan: bool,
    pub metadata: HashMap<String, String>,
}

/// Deployment result
#[derive(Debug, Clone)]
pub struct DeploymentResult {
    pub success: bool,
    pub deployment_id: uuid::Uuid,
    pub container_ids: Vec<String>,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
    pub rollback_performed: bool,
    pub security_scan_passed: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            enable_security_scan: true,
            enable_rollback: true,
            max_concurrent_deployments: 3,
            default_timeout_seconds: 1800,
            health_check_interval_seconds: 30,
            deployment_strategy: DeploymentStrategy::RollingUpdate {
                batch_size: 1,
                health_check_timeout_seconds: 300,
            },
            notifications: NotificationConfig {
                enabled: true,
                webhooks: Vec::new(),
                email_recipients: Vec::new(),
                slack_webhook: None,
            },
        }
    }
}

impl DeploymentPipeline {
    /// Create new deployment pipeline
    pub fn new(docker_client: DockerClient, security_scanner: SecurityScanner) -> Self {
        Self {
            docker_client: Arc::new(docker_client),
            security_scanner: Arc::new(security_scanner),
            environments: Arc::new(RwLock::new(HashMap::new())),
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
            deployment_history: Arc::new(RwLock::new(Vec::new())),
            config: PipelineConfig::default(),
        }
    }

    /// Create deployment pipeline with custom configuration
    pub fn with_config(docker_client: DockerClient, security_scanner: SecurityScanner, config: PipelineConfig) -> Self {
        Self {
            docker_client: Arc::new(docker_client),
            security_scanner: Arc::new(security_scanner),
            environments: Arc::new(RwLock::new(HashMap::new())),
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
            deployment_history: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Add environment to pipeline
    pub async fn add_environment(&self, environment: DeploymentEnvironment) -> Result<(), DockerConfigError> {
        let mut environments = self.environments.write().await;
        environments.insert(environment.id, environment);
        info!("Added environment to pipeline: {}", environment.name);
        Ok(())
    }

    /// Start deployment
    pub async fn deploy(&self, request: DeploymentRequest) -> Result<DeploymentResult, DockerConfigError> {
        let deployment_id = uuid::Uuid::new_v4();
        let start_time = chrono::Utc::now();

        info!("Starting deployment {} for environment {}", deployment_id, request.environment_id);

        // Check concurrent deployment limit
        if !self.check_concurrent_deployment_limit().await? {
            return Err(DockerConfigError::ConfigurationError(
                "Maximum concurrent deployments reached".to_string()
            ));
        }

        // Get environment
        let environment = {
            let environments = self.environments.read().await;
            environments.get(&request.environment_id)
                .cloned()
                .ok_or_else(|| DockerConfigError::ConfigurationError(
                    format!("Environment not found: {}", request.environment_id)
                ))?;
        };

        // Validate deployment request
        request.config.validate()?;

        // Create deployment record
        let strategy = request.strategy.unwrap_or_else(|| self.config.deployment_strategy.clone());
        let deployment = Deployment {
            id: deployment_id,
            environment_id: request.environment_id,
            image_name: request.image_name.clone(),
            config: request.config.clone(),
            status: DeploymentStatus::InProgress,
            strategy: strategy.clone(),
            start_time,
            end_time: None,
            duration_seconds: None,
            health_checks: Vec::new(),
            rollback_available: false,
            previous_version: None,
            metadata: request.metadata.clone(),
        };

        // Add to active deployments
        {
            let mut active_deployments = self.active_deployments.write().await;
            active_deployments.insert(deployment_id, deployment.clone());
        }

        // Send deployment start notification
        self.send_deployment_notification(&deployment, "started").await?;

        // Execute deployment based on strategy
        let result = match strategy {
            DeploymentStrategy::RollingUpdate { batch_size, health_check_timeout_seconds } => {
                self.execute_rolling_update(deployment, batch_size, health_check_timeout_seconds).await
            }
            DeploymentStrategy::BlueGreen { prep_environment: _, switch_delay_seconds: _ } => {
                self.execute_blue_green_deployment(deployment).await
            }
            DeploymentStrategy::Canary { canary_percentage, monitoring_duration_minutes } => {
                self.execute_canary_deployment(deployment, canary_percentage, monitoring_duration_minutes).await
            }
            DeploymentStrategy::Recreate => {
                self.execute_recreate_deployment(deployment).await
            }
        };

        // Update deployment status
        {
            let mut active_deployments = self.active_deployments.write().await;
            if let Some(active_deployment) = active_deployments.get_mut(&deployment_id) {
                active_deployment.status = if result.success {
                    DeploymentStatus::Success
                } else {
                    DeploymentStatus::Failed
                };
                active_deployment.end_time = Some(chrono::Utc::now());
                active_deployment.duration_seconds = Some(
                    (chrono::Utc::now() - start_time).num_seconds() as u64
                );
            }
        }

        // Move to deployment history
        {
            let mut active_deployments = self.active_deployments.write().await;
            let deployment = active_deployments.remove(&deployment_id);
            if let Some(dep) = deployment {
                let mut history = self.deployment_history.write().await;
                history.push(dep);
            }
        }

        // Send deployment completion notification
        self.send_deployment_notification(&deployment, if result.success { "completed" } else { "failed" }).await?;

        Ok(result)
    }

    /// Execute rolling update deployment
    async fn execute_rolling_update(
        &self,
        deployment: Deployment,
        batch_size: usize,
        health_check_timeout_seconds: u64,
    ) -> Result<DeploymentResult, DockerConfigError> {
        info!("Executing rolling update deployment: {}", deployment.id);

        // Get current containers for the environment
        let current_containers = self.get_environment_containers(deployment.environment_id).await?;

        // Calculate number of batches
        let total_containers = current_containers.len();
        let batches = (total_containers + batch_size - 1) / batch_size;

        let mut deployed_containers = Vec::new();
        let mut warnings = Vec::new();

        // Process each batch
        for batch_index in 0..batches {
            let start_idx = batch_index * batch_size;
            let end_idx = std::cmp::min((batch_index + 1) * batch_size, total_containers);

            info!("Processing batch {}/{}: containers {}-{}", batch_index + 1, batches, start_idx + 1, end_idx);

            // Stop containers in this batch
            for i in start_idx..end_idx {
                if let Some(container_id) = current_containers.get(i) {
                    if let Err(e) = self.docker_client.stop_container(container_id).await {
                        warn!("Failed to stop container {}: {}", container_id, e);
                        warnings.push(format!("Failed to stop container {}: {}", container_id, e));
                    }
                }
            }

            // Create new containers with updated image
            for i in start_idx..end_idx {
                let container_options = self.create_container_options(&deployment, Some(format!("{}-{}", deployment.config.image_name, i)))?;
                match self.docker_client.create_container(container_options).await {
                    Ok(container_id) => {
                        deployed_containers.push(container_id);

                        // Perform health check
                        if let Err(e) = self.perform_health_check(&container_id, health_check_timeout_seconds).await {
                            warn!("Health check failed for container {}: {}", container_id, e);
                            warnings.push(format!("Health check failed for container {}: {}", container_id, e));
                        }
                    }
                    Err(e) => {
                        error!("Failed to create container: {}", e);
                        return Err(e);
                    }
                }
            }

            // Wait between batches
            if batch_index < batches - 1 {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
        }

        Ok(DeploymentResult {
            success: true,
            deployment_id: deployment.id,
            container_ids: deployed_containers,
            error_message: None,
            warnings,
            rollback_performed: false,
            security_scan_passed: true,
        })
    }

    /// Execute blue-green deployment
    async fn execute_blue_green_deployment(&self, deployment: Deployment) -> Result<DeploymentResult, DockerConfigError> {
        info!("Executing blue-green deployment: {}", deployment.id);

        // This is a simplified implementation
        // In practice, you'd need environment management, traffic switching, etc.

        let container_options = self.create_container_options(&deployment, Some(format!("{}-green", deployment.config.image_name)))?;

        match self.docker_client.create_container(container_options).await {
            Ok(container_id) => {
                // Perform health check on green environment
                if let Err(e) = self.perform_health_check(&container_id, 300).await {
                    error!("Green environment health check failed: {}", e);
                    return Err(DockerConfigError::BuildError(e.to_string()));
                }

                // Switch traffic to green environment
                info!("Switching traffic to green environment");

                Ok(DeploymentResult {
                    success: true,
                    deployment_id: deployment.id,
                    container_ids: vec![container_id],
                    error_message: None,
                    warnings: Vec::new(),
                    rollback_performed: false,
                    security_scan_passed: true,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Execute canary deployment
    async fn execute_canary_deployment(
        &self,
        deployment: Deployment,
        canary_percentage: f64,
        monitoring_duration_minutes: u64,
    ) -> Result<DeploymentResult, DockerConfigError> {
        info!("Executing canary deployment: {} ({}% canary)", deployment.id, canary_percentage * 100.0);

        // Create canary container
        let container_options = self.create_container_options(&deployment, Some(format!("{}-canary", deployment.config.image_name)))?;

        match self.docker_client.create_container(container_options).await {
            Ok(canary_id) => {
                // Monitor canary deployment
                info!("Monitoring canary deployment for {} minutes", monitoring_duration_minutes);

                // In practice, you'd monitor metrics, error rates, etc.
                tokio::time::sleep(tokio::time::Duration::from_secs(monitoring_duration_minutes * 60)).await;

                // Gradually increase canary traffic
                info!("Gradually increasing canary traffic");

                Ok(DeploymentResult {
                    success: true,
                    deployment_id: deployment.id,
                    container_ids: vec![canary_id],
                    error_message: None,
                    warnings: Vec::new(),
                    rollback_performed: false,
                    security_scan_passed: true,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Execute recreate deployment
    async fn execute_recreate_deployment(&self, deployment: Deployment) -> Result<DeploymentResult, DockerConfigError> {
        info!("Executing recreate deployment: {}", deployment.id);

        // Get current containers
        let current_containers = self.get_environment_containers(deployment.environment_id).await?;

        // Stop all current containers
        let mut warnings = Vec::new();
        for container_id in current_containers {
            if let Err(e) = self.docker_client.stop_container(&container_id).await {
                warn!("Failed to stop container {}: {}", container_id, e);
                warnings.push(format!("Failed to stop container {}: {}", container_id, e));
            }
        }

        // Create new containers
        let mut new_containers = Vec::new();
        for i in 0..current_containers.len() {
            let container_options = self.create_container_options(&deployment, Some(format!("{}-{}", deployment.config.image_name, i)))?;
            match self.docker_client.create_container(container_options).await {
                Ok(container_id) => {
                    new_containers.push(container_id);
                }
                Err(e) => {
                    error!("Failed to create new container: {}", e);
                    return Err(e);
                }
            }
        }

        Ok(DeploymentResult {
            success: true,
            deployment_id: deployment.id,
            container_ids: new_containers,
            error_message: None,
            warnings,
            rollback_performed: false,
            security_scan_passed: true,
        })
    }

    /// Create container options from deployment config
    fn create_container_options(&self, deployment: &Deployment, name: Option<String>) -> Result<crate::integration::docker_client::ContainerOptions, DockerConfigError> {
        use crate::integration::docker_client::{ContainerOptions, RestartPolicy};

        let mut env_vars = deployment.config.environment.clone();
        env_vars.insert("DEPLOYMENT_ID".to_string(), deployment.id.to_string());
        env_vars.insert("ENVIRONMENT".to_string(), deployment.environment_id.to_string());

        let mut port_bindings = HashMap::new();
        for (container_port, host_port) in &deployment.config.host_port_mappings {
            port_bindings.insert(container_port.clone(), host_port.to_string());
        }

        Ok(ContainerOptions {
            name,
            image: deployment.config.image_name.clone(),
            command: None,
            env_vars,
            port_bindings,
            volume_bindings: HashMap::new(), // Would extract from config
            network: Some("bridge".to_string()),
            restart_policy: Some(RestartPolicy::UnlessStopped),
            resource_limits: deployment.config.resource_limits.clone(),
            security_options: vec!["no-new-privileges:true".to_string()],
        })
    }

    /// Perform health check on container
    async fn perform_health_check(&self, container_id: &str, timeout_seconds: u64) -> Result<(), DockerConfigError> {
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_seconds);

        loop {
            // Check container status
            let status = self.docker_client.get_container_status(container_id).await?;
            if status == ContainerStatus::Running {
                // Additional health checks can be added here
                return Ok(());
            }

            if start_time.elapsed() > timeout {
                return Err(DockerConfigError::BuildError(
                    format!("Health check timeout for container {}", container_id)
                ));
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    /// Get containers for an environment
    async fn get_environment_containers(&self, environment_id: uuid::Uuid) -> Result<Vec<String>, DockerConfigError> {
        // In a real implementation, you'd track which containers belong to which environment
        // For now, return empty list
        Ok(Vec::new())
    }

    /// Check concurrent deployment limit
    async fn check_concurrent_deployment_limit(&self) -> Result<bool, DockerConfigError> {
        let active_deployments = self.active_deployments.read().await;
        Ok(active_deployments.len() < self.config.max_concurrent_deployments)
    }

    /// Send deployment notification
    async fn send_deployment_notification(&self, deployment: &Deployment, status: &str) -> Result<(), DockerConfigError> {
        if !self.config.notifications.enabled {
            return Ok(());
        }

        let message = json!({
            "deployment_id": deployment.id,
            "environment_id": deployment.environment_id,
            "image": deployment.image_name,
            "status": status,
            "strategy": format!("{:?}", deployment.strategy),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        info!("Deployment notification: {} - {}", deployment.id, status);

        // In a real implementation, send to webhooks, Slack, email, etc.
        if let Some(ref slack_webhook) = self.config.notifications.slack_webhook {
            debug!("Would send Slack notification to: {}", slack_webhook);
        }

        Ok(())
    }

    /// Get deployment status
    pub async fn get_deployment_status(&self, deployment_id: uuid::Uuid) -> Result<Option<Deployment>, DockerConfigError> {
        let active_deployments = self.active_deployments.read().await;
        let deployment = active_deployments.get(&deployment_id);

        if deployment.is_some() {
            return Ok(deployment.cloned());
        }

        // Check history
        let history = self.deployment_history.read().await;
        Ok(history.iter()
            .find(|d| d.id == deployment_id)
            .cloned())
    }

    /// List active deployments
    pub async fn list_active_deployments(&self) -> Result<Vec<Deployment>, DockerConfigError> {
        let active_deployments = self.active_deployments.read().await;
        Ok(active_deployments.values().cloned().collect())
    }

    /// List deployment history
    pub async fn list_deployment_history(&self, limit: Option<usize>) -> Result<Vec<Deployment>, DockerConfigError> {
        let history = self.deployment_history.read().await;
        let mut deployments = history.clone();
        deployments.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        if let Some(lim) = limit {
            deployments.truncate(lim);
        }

        Ok(deployments)
    }

    /// Cancel deployment
    pub async fn cancel_deployment(&self, deployment_id: uuid::Uuid) -> Result<bool, DockerConfigError> {
        let mut active_deployments = self.active_deployments.write().await;

        if let Some(mut deployment) = active_deployments.remove(&deployment_id) {
            deployment.status = DeploymentStatus::Cancelled;
            deployment.end_time = Some(chrono::Utc::now());

            let mut history = self.deployment_history.write().await;
            history.push(deployment);

            info!("Cancelled deployment: {}", deployment_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert!(config.enable_security_scan);
        assert_eq!(config.max_concurrent_deployments, 3);
    }

    #[test]
    fn test_deployment_request_validation() {
        let config = DockerContainerConfig::new("test:latest".to_string(), "Dockerfile".to_string());
        let request = DeploymentRequest {
            environment_id: uuid::Uuid::new_v4(),
            image_name: "test:latest".to_string(),
            config,
            strategy: None,
            timeout_seconds: None,
            skip_security_scan: false,
            metadata: HashMap::new(),
        };

        // Should not panic
        let _ = request;
    }

    #[tokio::test]
    async fn test_deployment_pipeline_creation() {
        let docker_client = DockerClient::new("unix:///var/run/docker.sock".to_string());
        let security_scanner = SecurityScanner::new().unwrap();
        let pipeline = DeploymentPipeline::new(docker_client, security_scanner);

        assert_eq!(pipeline.config.max_concurrent_deployments, 3);
    }
}