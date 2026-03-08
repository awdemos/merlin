//! Merlin AI Router - Hardened Docker Deployment Service
//!
//! This service provides hardened Docker container deployment with security scanning,
//! resource management, and monitoring capabilities.

use merlin::services::{DockerConfigService, ContainerStateService, DockerErrorService};
use merlin::models::{DockerContainerConfig, ResourceLimits, SecurityProfile};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;
use tracing::{info, error, warn};
use tracing_subscriber;

/// Main configuration structure
#[derive(serde::Deserialize)]
struct Config {
    /// Server port
    pub port: u16,

    /// Log level
    pub log_level: String,

    /// Docker configuration
    pub docker: DockerConfig,
}

/// Docker-specific configuration
#[derive(serde::Deserialize)]
struct DockerConfig {
    /// Default registry URL
    pub registry_url: Option<String>,

    /// Security scanning enabled
    pub security_scanning: bool,

    /// Default resource limits
    pub default_limits: Option<ResourceLimits>,

    /// Default security profile
    pub default_security: Option<SecurityProfile>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    init_logging();

    info!("Starting Merlin AI Router - Hardened Docker Deployment Service");

    // Load configuration
    let config = load_configuration()?;

    info!("Configuration loaded successfully");
    info!("Server will start on port {}", config.port);

    // Initialize services
    let docker_config_service = Arc::new(DockerConfigService::new()?);
    let container_state_service = Arc::new(ContainerStateService::new()?);
    let error_service = Arc::new(DockerErrorService::new().await?);

    info!("Services initialized successfully");

    // Combine all routes
    let routes = build_routes(docker_config_service, container_state_service, error_service);

    // Start server
    info!("Starting server on 0.0.0.0:{}", config.port);
    warp::serve(routes)
        .run(([0, 0, 0, 0], config.port))
        .await;

    Ok(())
}

/// Initialize logging with structured formatting
fn init_logging() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();
}

/// Load configuration from environment and config files
fn load_configuration() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    // Default configuration
    let mut config = Config {
        port: 8080,
        log_level: "info".to_string(),
        docker: DockerConfig {
            registry_url: Some("docker.io".to_string()),
            security_scanning: true,
            default_limits: None,
            default_security: None,
        },
    };

    // Load from environment variables
    if let Ok(port) = std::env::var("MERLIN_PORT") {
        config.port = port.parse()
            .map_err(|e| format!("Invalid port number: {}", e))?;
    }

    if let Ok(log_level) = std::env::var("MERLIN_LOG_LEVEL") {
        config.log_level = log_level;
    }

    // Try to load from config file
    if let Ok(config_path) = std::env::var("MERLIN_CONFIG_FILE") {
        if std::path::Path::new(&config_path).exists() {
            let config_content = std::fs::read_to_string(config_path)?;
            let file_config: Config = serde_json::from_str(&config_content)
                .map_err(|e| format!("Failed to parse config file: {}", e))?;

            // Merge configurations
            config.port = file_config.port;
            config.log_level = file_config.log_level;
            config.docker = file_config.docker;
        }
    }

    Ok(config)
}

/// Build all API routes
fn build_routes(
    docker_config_service: Arc<DockerConfigService>,
    container_state_service: Arc<ContainerStateService>,
    error_service: Arc<DockerErrorService>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {

    // CORS headers
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]);

    // Health check endpoint
    let health = warp::path!("health")
        .and(warp::get())
        .map(|| {
            warp::reply::with_status(
                json!({"status": "healthy", "timestamp": chrono::Utc::now().to_rfc3339()}),
                warp::http::StatusCode::OK,
            )
        });

    // API version info
    let version = warp::path!("version")
        .and(warp::get())
        .map(|| {
            warp::reply::with_status(
                json!({
                    "version": "1.0.0",
                    "name": "Merlin AI Router - Docker Deployment Service",
                    "description": "Hardened Docker container deployment with security scanning",
                    "build_timestamp": env!("VERGEN_BUILD_TIMESTAMP"),
                    "git_sha": env!("VERGEN_GIT_SHA"),
                }),
                warp::http::StatusCode::OK,
            )
        });

    // Docker configuration routes
    let docker_routes = docker_config_service.build_endpoint()
        .or(docker_config_service.scan_endpoint())
        .or(docker_config_service.validate_endpoint())
        .or(docker_config_service.health_endpoint());

    // Container state routes
    let container_routes = container_state_service.status_endpoint()
        .or(container_state_service.metrics_endpoint())
        .or(container_state_service.events_endpoint())
        .or(container_state_service.operations_endpoint());

    // Error service routes
    let error_routes = warp::path!("api" / "v1" / "errors")
        .and(warp::get())
        .and(warp::any().map(move || error_service.clone()))
        .and_then(handle_error_logs);

    // All routes combined
    let api_routes = warp::path!("api" / "v1" / ..)
        .and(docker_routes
            .or(container_routes)
            .or(error_routes));

    // Combine all routes with CORS
    health
        .or(version)
        .or(api_routes)
        .with(cors)
        .with(warp::log("merlin_docker_service"))
}

/// Handle error log requests
async fn handle_error_logs(
    error_service: Arc<DockerErrorService>,
) -> Result<impl warp::Reply, warp::Rejection> {
    match error_service.get_error_summary().await {
        Ok(summary) => {
            Ok(warp::reply::with_status(json!(summary), warp::http::StatusCode::OK))
        }
        Err(e) => {
            error!("Failed to get error summary: {}", e);
            Ok(warp::reply::with_status(
                json!({"error": "Failed to retrieve error logs"}),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Configuration module
mod config {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AppConfig {
        pub server: ServerConfig,
        pub docker: DockerConfig,
        pub security: SecurityConfig,
        pub monitoring: MonitoringConfig,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ServerConfig {
        pub port: u16,
        pub host: String,
        pub workers: usize,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DockerConfig {
        pub registry_url: String,
        pub default_timeout_seconds: u64,
        pub max_concurrent_builds: usize,
        pub cleanup_policy: CleanupPolicy,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CleanupPolicy {
        pub max_images: usize,
        pub max_age_days: u32,
        pub cleanup_interval_hours: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SecurityConfig {
        pub enable_scanning: bool,
        pub required_security_level: String,
        pub allowed_registries: Vec<String>,
        pub scan_timeout_seconds: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MonitoringConfig {
        pub metrics_enabled: bool,
        pub health_check_interval_seconds: u64,
        pub max_log_age_days: u32,
    }

    impl Default for AppConfig {
        fn default() -> Self {
            Self {
                server: ServerConfig {
                    port: 8080,
                    host: "0.0.0.0".to_string(),
                    workers: 4,
                },
                docker: DockerConfig {
                    registry_url: "docker.io".to_string(),
                    default_timeout_seconds: 300,
                    max_concurrent_builds: 3,
                    cleanup_policy: CleanupPolicy {
                        max_images: 50,
                        max_age_days: 30,
                        cleanup_interval_hours: 24,
                    },
                },
                security: SecurityConfig {
                    enable_scanning: true,
                    required_security_level: "high".to_string(),
                    allowed_registries: vec!["docker.io".to_string()],
                    scan_timeout_seconds: 600,
                },
                monitoring: MonitoringConfig {
                    metrics_enabled: true,
                    health_check_interval_seconds: 30,
                    max_log_age_days: 7,
                },
            }
        }
    }
}

/// Error handling module
mod error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum ServiceError {
        #[error("Configuration error: {0}")]
        Configuration(String),

        #[error("Docker operation failed: {0}")]
        DockerOperation(String),

        #[error("Security validation failed: {0}")]
        SecurityValidation(String),

        #[error("Resource limit exceeded: {0}")]
        ResourceLimit(String),

        #[error("Network error: {0}")]
        Network(String),

        #[error("Internal server error: {0}")]
        Internal(String),
    }

    impl ServiceError {
        pub fn error_code(&self) -> u16 {
            match self {
                ServiceError::Configuration(_) => 400,
                ServiceError::DockerOperation(_) => 500,
                ServiceError::SecurityValidation(_) => 403,
                ServiceError::ResourceLimit(_) => 429,
                ServiceError::Network(_) => 503,
                ServiceError::Internal(_) => 500,
            }
        }

        pub fn to_json(&self) -> serde_json::Value {
            serde_json::json!({
                "error": self.to_string(),
                "code": self.error_code(),
                "type": std::mem::discriminant(self).name(),
            })
        }
    }
}

/// Request/response processor module
mod processor {
    use crate::error::ServiceError;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BuildRequest {
        pub image_name: String,
        pub dockerfile_path: String,
        pub build_args: Option<Vec<(String, String)>>,
        pub tags: Option<Vec<String>>,
        pub security_context: Option<serde_json::Value>,
        pub resource_limits: Option<serde_json::Value>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BuildResponse {
        pub status: String,
        pub image_id: String,
        pub build_time: String,
        pub config_id: uuid::Uuid,
        pub logs: Vec<String>,
        pub warnings: Vec<String>,
        pub errors: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ScanRequest {
        pub image_name: String,
        pub scan_types: Vec<String>,
        pub severity_threshold: Option<String>,
        pub timeout_seconds: Option<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ScanResponse {
        pub status: String,
        pub image: String,
        pub scan_id: uuid::Uuid,
        pub scan_time: String,
        pub vulnerabilities: Vec<Vulnerability>,
        pub configuration_checks: serde_json::Value,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Vulnerability {
        pub id: String,
        pub severity: String,
        pub package: String,
        pub version: String,
        pub fixed_version: Option<String>,
        pub description: Option<String>,
    }

    impl BuildRequest {
        pub fn validate(&self) -> Result<(), ServiceError> {
            if self.image_name.is_empty() {
                return Err(ServiceError::Configuration("Image name cannot be empty".to_string()));
            }

            if self.dockerfile_path.is_empty() {
                return Err(ServiceError::Configuration("Dockerfile path cannot be empty".to_string()));
            }

            // Validate image name format
            if !self.image_name.chars().all(|c| c.is_alphanumeric() || c == ':' || c == '-' || c == '_' || c == '.') {
                return Err(ServiceError::Configuration("Invalid image name format".to_string()));
            }

            Ok(())
        }
    }

    impl ScanRequest {
        pub fn validate(&self) -> Result<(), ServiceError> {
            if self.image_name.is_empty() {
                return Err(ServiceError::Configuration("Image name cannot be empty".to_string()));
            }

            if self.scan_types.is_empty() {
                return Err(ServiceError::Configuration("At least one scan type must be specified".to_string()));
            }

            // Validate scan types
            let valid_scan_types = ["vulnerability", "configuration", "malware", "license"];
            for scan_type in &self.scan_types {
                if !valid_scan_types.contains(&scan_type.as_str()) {
                    return Err(ServiceError::Configuration(format!("Invalid scan type: {}", scan_type)));
                }
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = config::AppConfig::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.docker.registry_url, "docker.io");
        assert!(config.security.enable_scanning);
    }

    #[test]
    fn test_error_codes() {
        let config_error = error::ServiceError::Configuration("test".to_string());
        assert_eq!(config_error.error_code(), 400);

        let security_error = error::ServiceError::SecurityValidation("test".to_string());
        assert_eq!(security_error.error_code(), 403);
    }

    #[test]
    fn test_build_request_validation() {
        let valid_request = processor::BuildRequest {
            image_name: "test:latest".to_string(),
            dockerfile_path: "Dockerfile".to_string(),
            build_args: None,
            tags: None,
            security_context: None,
            resource_limits: None,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = processor::BuildRequest {
            image_name: "".to_string(),
            dockerfile_path: "Dockerfile".to_string(),
            build_args: None,
            tags: None,
            security_context: None,
            resource_limits: None,
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_scan_request_validation() {
        let valid_request = processor::ScanRequest {
            image_name: "test:latest".to_string(),
            scan_types: vec!["vulnerability".to_string()],
            severity_threshold: None,
            timeout_seconds: None,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = processor::ScanRequest {
            image_name: "test:latest".to_string(),
            scan_types: vec!["invalid_scan_type".to_string()],
            severity_threshold: None,
            timeout_seconds: None,
        };
        assert!(invalid_request.validate().is_err());
    }
}