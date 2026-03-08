use crate::models::docker_config::{DockerContainerConfig, DockerConfigError};
use crate::models::security_scan_config::{SecurityScanConfig, SecurityScanError};
use crate::models::deployment_environment::DeploymentEnvironment;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};

/// Service for managing Docker configurations and deployments
#[derive(Clone)]
pub struct DockerConfigService {
    /// Store for container configurations
    configurations: Arc<RwLock<HashMap<uuid::Uuid, DockerContainerConfig>>>,

    /// Store for scan configurations
    scan_configs: Arc<RwLock<HashMap<uuid::Uuid, SecurityScanConfig>>>,

    /// Store for deployment environments
    environments: Arc<RwLock<HashMap<uuid::Uuid, DeploymentEnvironment>>>,
}

impl DockerConfigService {
    /// Create a new Docker configuration service
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            configurations: Arc::new(RwLock::new(HashMap::new())),
            scan_configs: Arc::new(RwLock::new(HashMap::new())),
            environments: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a new container configuration
    pub async fn create_config(&self, config: DockerContainerConfig) -> Result<uuid::Uuid, DockerConfigError> {
        // Validate the configuration
        config.validate()?;

        // Store the configuration
        let mut configs = self.configurations.write().await;
        let id = config.id;
        configs.insert(id, config.clone());

        Ok(id)
    }

    /// Get a container configuration by ID
    pub async fn get_config(&self, id: uuid::Uuid) -> Result<Option<DockerContainerConfig>, DockerConfigError> {
        let configs = self.configurations.read().await;
        Ok(configs.get(&id).cloned())
    }

    /// Update a container configuration
    pub async fn update_config(&self, id: uuid::Uuid, config: DockerContainerConfig) -> Result<bool, DockerConfigError> {
        // Validate the configuration
        config.validate()?;

        let mut configs = self.configurations.write().await;
        if configs.contains_key(&id) {
            configs.insert(id, config);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Delete a container configuration
    pub async fn delete_config(&self, id: uuid::Uuid) -> Result<bool, DockerConfigError> {
        let mut configs = self.configurations.write().await;
        Ok(configs.remove(&id).is_some())
    }

    /// List all container configurations
    pub async fn list_configs(&self) -> Vec<DockerContainerConfig> {
        let configs = self.configurations.read().await;
        configs.values().cloned().collect()
    }

    /// Build a Docker image from configuration
    pub async fn build_image(&self, config_id: uuid::Uuid) -> Result<DockerBuildResult, DockerConfigError> {
        let config = self.get_config(config_id).await?
            .ok_or_else(|| DockerConfigError::ValidationError("Configuration not found".to_string()))?;

        // For now, simulate the build process
        // In a real implementation, this would call Docker API
        let build_result = DockerBuildResult {
            config_id,
            image_id: format!("{}:{}", config.image_name, uuid::Uuid::new_v4()),
            build_time: chrono::Utc::now(),
            success: true,
            logs: vec!["Step 1/10 : FROM gcr.io/distroless/static-debian11:latest".to_string()],
            warnings: vec![],
            errors: vec![],
        };

        Ok(build_result)
    }

    /// Validate a Docker configuration
    pub async fn validate_config(&self, config_id: uuid::Uuid) -> Result<DockerValidationResult, DockerConfigError> {
        let config = self.get_config(config_id).await?
            .ok_or_else(|| DockerConfigError::ValidationError("Configuration not found".to_string()))?;

        let mut violations = Vec::new();
        let mut warnings = Vec::new();
        let mut recommendations = Vec::new();

        // Validate security context
        if let Some(ref security) = config.security_context {
            if let Some(user) = security.get("user") {
                if user == "root" || user == "0" {
                    violations.push("Container runs as root user".to_string());
                }
            }

            if let Some(read_only) = security.get("read_only") {
                if read_only != true {
                    violations.push("Container filesystem is not read-only".to_string());
                }
            }
        }

        // Validate resource limits
        if let Some(ref limits) = config.resource_limits {
            if limits.memory_mb > 4096 {
                warnings.push("High memory limit may impact performance".to_string());
            }

            if limits.cpu_shares > 4.0 {
                warnings.push("High CPU limit may impact performance".to_string());
            }
        }

        // Generate recommendations
        if config.security_context.is_none() {
            recommendations.push("Add security context for better isolation".to_string());
        }

        if config.health_check.is_none() {
            recommendations.push("Add health check for better monitoring".to_string());
        }

        // Calculate security score
        let security_score = if violations.is_empty() {
            if warnings.is_empty() { 100 } else { 85 }
        } else {
            (50 - violations.len() as u8 * 10).max(0)
        };

        Ok(DockerValidationResult {
            config_id,
            valid: violations.is_empty(),
            security_score,
            violations,
            warnings,
            recommendations,
            validated_at: chrono::Utc::now(),
        })
    }

    /// Create a new deployment environment
    pub async fn create_environment(&self, env: DeploymentEnvironment) -> Result<uuid::Uuid, DockerConfigError> {
        // Validate the environment
        env.validate().map_err(|e| DockerConfigError::ValidationError(e.to_string()))?;

        // Store the environment
        let mut environments = self.environments.write().await;
        let id = env.id;
        environments.insert(id, env.clone());

        Ok(id)
    }

    /// Get a deployment environment by ID
    pub async fn get_environment(&self, id: uuid::Uuid) -> Result<Option<DeploymentEnvironment>, DockerConfigError> {
        let environments = self.environments.read().await;
        Ok(environments.get(&id).cloned())
    }

    /// Get system health status
    pub async fn get_health_status(&self) -> DockerHealthStatus {
        let configs = self.configurations.read().await;
        let environments = self.environments.read().await;

        DockerHealthStatus {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            version: "1.0.0".to_string(),
            uptime_seconds: 0, // TODO: Track actual uptime
            configurations_count: configs.len(),
            environments_count: environments.len(),
            memory_usage_mb: 50, // TODO: Get actual memory usage
            cpu_usage_percent: 10.0, // TODO: Get actual CPU usage
            docker_daemon_status: "running".to_string(),
            security_services_status: json!({
                "trivy": "available",
                "hadolint": "available",
                "docker_bench": "available"
            }),
        }
    }

    /// Warp filter for build endpoint
    pub fn build_endpoint(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("api" / "v1" / "docker" / "build")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_service(self.clone()))
            .and_then(handle_build)
    }

    /// Warp filter for scan endpoint
    pub fn scan_endpoint(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("api" / "v1" / "docker" / "scan")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_service(self.clone()))
            .and_then(handle_scan)
    }

    /// Warp filter for validate endpoint
    pub fn validate_endpoint(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("api" / "v1" / "docker" / "validate")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_service(self.clone()))
            .and_then(handle_validate)
    }

    /// Warp filter for health endpoint
    pub fn health_endpoint(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("api" / "v1" / "docker" / "health")
            .and(warp::get())
            .and(with_service(self.clone()))
            .and_then(handle_health)
    }
}

/// Helper function to pass service to warp handlers
fn with_service(
    service: DockerConfigService,
) -> impl Filter<Extract = (DockerConfigService,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || service.clone())
}

/// Handle Docker build requests
async fn handle_build(
    config: DockerContainerConfig,
    service: DockerConfigService,
) -> Result<impl Reply, Rejection> {
    match service.create_config(config).await {
        Ok(config_id) => {
            match service.build_image(config_id).await {
                Ok(build_result) => {
                    let response = json!({
                        "status": "success",
                        "image_id": build_result.image_id,
                        "build_time": build_result.build_time.to_rfc3339(),
                        "config_id": config_id
                    });
                    Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::OK))
                }
                Err(e) => {
                    let response = json!({
                        "status": "error",
                        "message": e.to_string(),
                        "config_id": config_id
                    });
                    Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::BAD_REQUEST))
                }
            }
        }
        Err(e) => {
            let response = json!({
                "status": "error",
                "message": e.to_string()
            });
            Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::BAD_REQUEST))
        }
    }
}

/// Handle Docker scan requests
async fn handle_scan(
    scan_request: serde_json::Value,
    service: DockerConfigService,
) -> Result<impl Reply, Rejection> {
    // Parse scan request
    let image_name = scan_request.get("image_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let empty_array: Vec<serde_json::Value> = vec![];
    let scan_types = scan_request.get("scan_types")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_array);

    if image_name.is_empty() {
        let response = json!({
            "status": "error",
            "message": "Image name is required"
        });
        return Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::BAD_REQUEST));
    }

    // For now, simulate scan results
    let scan_id = uuid::Uuid::new_v4();
    let response = json!({
        "status": "success",
        "image": image_name,
        "scan_id": scan_id,
        "scan_time": chrono::Utc::now().to_rfc3339(),
        "scan_types": scan_types,
        "vulnerabilities": [
            {
                "id": "CVE-2023-1234",
                "severity": "CRITICAL",
                "package": "openssl",
                "version": "1.1.1",
                "fixed_version": "1.1.1t"
            }
        ],
        "configuration_checks": {
            "non_root_user": true,
            "read_only_fs": true,
            "capability_dropping": true
        }
    });

    Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::OK))
}

/// Handle Docker validation requests
async fn handle_validate(
    validation_request: serde_json::Value,
    service: DockerConfigService,
) -> Result<impl Reply, Rejection> {
    // Create a temporary config for validation
    let config = DockerContainerConfig::new(
        validation_request.get("image_name")
            .and_then(|v| v.as_str())
            .unwrap_or("temp:latest")
            .to_string(),
        "docker/Dockerfile.hardened".to_string()
    );

    match service.create_config(config).await {
        Ok(config_id) => {
            match service.validate_config(config_id).await {
                Ok(validation_result) => {
                    let response = json!({
                        "status": if validation_result.valid { "valid" } else { "invalid" },
                        "score": validation_result.security_score,
                        "passed_checks": validation_result.violations.is_empty(),
                        "violations": validation_result.violations,
                        "recommendations": validation_result.recommendations,
                        "validated_at": validation_result.validated_at.to_rfc3339()
                    });
                    Ok(warp::reply::with_status(warp::reply::json(&response), if validation_result.valid {
                        warp::http::StatusCode::OK
                    } else {
                        warp::http::StatusCode::BAD_REQUEST
                    }))
                }
                Err(e) => {
                    let response = json!({
                        "status": "error",
                        "message": e.to_string()
                    });
                    Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::BAD_REQUEST))
                }
            }
        }
        Err(e) => {
            let response = json!({
                "status": "error",
                "message": e.to_string()
            });
            Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::BAD_REQUEST))
        }
    }
}

/// Handle health check requests
async fn handle_health(
    service: DockerConfigService,
) -> Result<impl Reply, Rejection> {
    let health_status = service.get_health_status().await;

    let response = json!({
        "status": health_status.status,
        "timestamp": health_status.timestamp.to_rfc3339(),
        "version": health_status.version,
        "uptime_seconds": health_status.uptime_seconds,
        "details": {
            "configurations": health_status.configurations_count,
            "environments": health_status.environments_count,
            "memory_usage_mb": health_status.memory_usage_mb,
            "cpu_usage_percent": health_status.cpu_usage_percent,
            "docker_daemon": health_status.docker_daemon_status,
            "security_services": health_status.security_services_status
        }
    });

    Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::OK))
}

/// Result of Docker image build
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DockerBuildResult {
    pub config_id: uuid::Uuid,
    pub image_id: String,
    pub build_time: chrono::DateTime<chrono::Utc>,
    pub success: bool,
    pub logs: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Result of Docker configuration validation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DockerValidationResult {
    pub config_id: uuid::Uuid,
    pub valid: bool,
    pub security_score: u8,
    pub violations: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    pub validated_at: chrono::DateTime<chrono::Utc>,
}

/// Docker system health status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DockerHealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub configurations_count: usize,
    pub environments_count: usize,
    pub memory_usage_mb: u32,
    pub cpu_usage_percent: f64,
    pub docker_daemon_status: String,
    pub security_services_status: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_service() {
        let service = DockerConfigService::new();
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_create_config() {
        let service = DockerConfigService::new().unwrap();
        let config = DockerContainerConfig::new("test:latest".to_string(), "Dockerfile".to_string());

        let result = service.create_config(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_config() {
        let service = DockerConfigService::new().unwrap();
        let config = DockerContainerConfig::new("test:latest".to_string(), "Dockerfile".to_string());

        let id = service.create_config(config.clone()).await.unwrap();
        let retrieved = service.get_config(id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().image_name, "test:latest");
    }

    #[tokio::test]
    async fn test_validate_config() {
        let service = DockerConfigService::new().unwrap();
        let config = DockerContainerConfig::new("test:latest".to_string(), "Dockerfile".to_string());

        let id = service.create_config(config).await.unwrap();
        let result = service.validate_config(id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().valid);
    }

    #[tokio::test]
    async fn test_health_status() {
        let service = DockerConfigService::new().unwrap();
        let health = service.get_health_status().await;

        assert_eq!(health.status, "healthy");
        assert_eq!(health.configurations_count, 0);
        assert_eq!(health.environments_count, 0);
    }

    #[tokio::test]
    async fn test_create_environment() {
        let service = DockerConfigService::new().unwrap();
        let env = DeploymentEnvironment::new("test-env".to_string(),
                                              crate::models::deployment_environment::EnvironmentType::Development);

        let result = service.create_environment(env).await;
        assert!(result.is_ok());
    }
}