//! Docker API client integration for Merlin AI Router
//!
//! Provides high-level integration with Docker Engine API for container management,
//! image building, security scanning, and monitoring operations.

use crate::models::docker_config::{DockerContainerConfig, DockerConfigError};
use crate::models::container_state::{ContainerState, ContainerStatus, ContainerMetrics};
use crate::models::resource_limits::ResourceLimits;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;
use tracing::{info, warn, error, debug};

/// Docker client for container operations
pub struct DockerClient {
    /// Docker host address
    host: String,

    /// Default timeout for operations
    default_timeout: std::time::Duration,

    /// Authentication configuration
    auth: Option<DockerAuth>,
}

/// Docker authentication configuration
#[derive(Debug, Clone)]
pub struct DockerAuth {
    pub username: String,
    pub password: String,
    pub server_address: String,
}

/// Build context configuration
#[derive(Debug, Clone)]
pub struct BuildContext {
    pub dockerfile_path: String,
    pub context_path: String,
    pub build_args: Vec<(String, String)>,
    pub tags: Vec<String>,
    pub target: Option<String>,
    pub network_mode: Option<String>,
}

/// Container create options
#[derive(Debug, Clone)]
pub struct ContainerOptions {
    pub name: Option<String>,
    pub image: String,
    pub command: Option<Vec<String>>,
    pub env_vars: HashMap<String, String>,
    pub port_bindings: HashMap<String, String>,
    pub volume_bindings: HashMap<String, String>,
    pub network: Option<String>,
    pub restart_policy: Option<RestartPolicy>,
    pub resource_limits: Option<ResourceLimits>,
    pub security_options: Vec<String>,
}

/// Container restart policy
#[derive(Debug, Clone)]
pub enum RestartPolicy {
    No,
    OnFailure,
    Always,
    UnlessStopped,
}

impl DockerClient {
    /// Create new Docker client
    pub fn new(host: String) -> Self {
        Self {
            host,
            default_timeout: std::time::Duration::from_secs(300),
            auth: None,
        }
    }

    /// Create Docker client with authentication
    pub fn with_auth(host: String, auth: DockerAuth) -> Self {
        Self {
            host,
            default_timeout: std::time::Duration::from_secs(300),
            auth: Some(auth),
        }
    }

    /// Set default timeout for operations
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Build Docker image from Dockerfile
    pub async fn build_image(&self, context: BuildContext) -> Result<DockerBuildResult, DockerConfigError> {
        info!("Building Docker image from: {}", context.dockerfile_path);

        // Validate build context
        if !Path::new(&context.dockerfile_path).exists() {
            return Err(DockerConfigError::ValidationError(
                format!("Dockerfile not found: {}", context.dockerfile_path)
            ));
        }

        if !Path::new(&context.context_path).exists() {
            return Err(DockerConfigError::ValidationError(
                format!("Build context not found: {}", context.context_path)
            ));
        }

        // Prepare docker build command
        let mut cmd = Command::new("docker");
        cmd.arg("build")
            .arg("-f").arg(&context.dockerfile_path)
            .arg(&context.context_path);

        // Add build arguments
        for (key, value) in &context.build_args {
            cmd.arg("--build-arg").arg(format!("{}={}", key, value));
        }

        // Add tags
        for tag in &context.tags {
            cmd.arg("-t").arg(tag);
        }

        // Add optional build arguments
        if let Some(ref target) = context.target {
            cmd.arg("--target").arg(target);
        }

        if let Some(ref network) = context.network_mode {
            cmd.arg("--network").arg(network);
        }

        // Execute build command
        let output = cmd.output().await.map_err(|e| {
            DockerConfigError::BuildError(format!("Failed to execute docker build: {}", e))
        })?;

        // Parse build result
        let build_result = if output.status.success() {
            let image_id = self.extract_image_id(&output.stdout).unwrap_or_else(|| "unknown".to_string());
            let logs = String::from_utf8_lossy(&output.stdout).lines().map(|s| s.to_string()).collect();

            DockerBuildResult {
                success: true,
                image_id,
                build_time: chrono::Utc::now(),
                logs,
                warnings: Vec::new(),
                errors: Vec::new(),
            }
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            let errors = vec![error_output.to_string()];

            error!("Docker build failed: {}", error_output);

            DockerBuildResult {
                success: false,
                image_id: String::new(),
                build_time: chrono::Utc::now(),
                logs: Vec::new(),
                warnings: Vec::new(),
                errors,
            }
        };

        Ok(build_result)
    }

    /// Create and start container
    pub async fn create_container(&self, options: ContainerOptions) -> Result<String, DockerConfigError> {
        info!("Creating container from image: {}", options.image);

        // Prepare docker run command
        let mut cmd = Command::new("docker");
        cmd.arg("run").arg("-d"); // detached mode

        // Add container name
        if let Some(ref name) = options.name {
            cmd.arg("--name").arg(name);
        }

        // Add environment variables
        for (key, value) in &options.env_vars {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }

        // Add port bindings
        for (container_port, host_port) in &options.port_bindings {
            cmd.arg("-p").arg(format!("{}:{}", host_port, container_port));
        }

        // Add volume bindings
        for (host_path, container_path) in &options.volume_bindings {
            cmd.arg("-v").arg(format!("{}:{}", host_path, container_path));
        }

        // Add network
        if let Some(ref network) = options.network {
            cmd.arg("--network").arg(network);
        }

        // Add restart policy
        if let Some(ref policy) = options.restart_policy {
            match policy {
                RestartPolicy::No => cmd.arg("--restart=no"),
                RestartPolicy::OnFailure => cmd.arg("--restart=on-failure"),
                RestartPolicy::Always => cmd.arg("--restart=always"),
                RestartPolicy::UnlessStopped => cmd.arg("--restart=unless-stopped"),
            };
        }

        // Add resource limits
        if let Some(ref limits) = options.resource_limits {
            if limits.memory_mb > 0 {
                cmd.arg("--memory").arg(format!("{}m", limits.memory_mb));
            }
            if limits.cpu_shares > 0.0 {
                cmd.arg("--cpus").arg(format!("{}", limits.cpu_shares));
            }
            if limits.pids_limit > 0 {
                cmd.arg("--pids-limit").arg(format!("{}", limits.pids_limit));
            }
        }

        // Add security options
        for security_opt in &options.security_options {
            cmd.arg("--security-opt").arg(security_opt);
        }

        // Add image and command
        cmd.arg(&options.image);
        if let Some(ref command) = options.command {
            cmd.args(command);
        }

        // Execute create command
        let output = cmd.output().await.map_err(|e| {
            DockerConfigError::BuildError(format!("Failed to execute docker run: {}", e))
        })?;

        if output.status.success() {
            let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            info!("Container created successfully: {}", container_id);
            Ok(container_id)
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            error!("Failed to create container: {}", error_output);
            Err(DockerConfigError::BuildError(error_output.to_string()))
        }
    }

    /// Stop container
    pub async fn stop_container(&self, container_id: &str) -> Result<bool, DockerConfigError> {
        info!("Stopping container: {}", container_id);

        let output = Command::new("docker")
            .arg("stop")
            .arg(container_id)
            .output()
            .await
            .map_err(|e| {
                DockerConfigError::BuildError(format!("Failed to execute docker stop: {}", e))
            })?;

        if output.status.success() {
            info!("Container stopped successfully: {}", container_id);
            Ok(true)
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to stop container {}: {}", container_id, error_output);
            Ok(false)
        }
    }

    /// Start container
    pub async fn start_container(&self, container_id: &str) -> Result<bool, DockerConfigError> {
        info!("Starting container: {}", container_id);

        let output = Command::new("docker")
            .arg("start")
            .arg(container_id)
            .output()
            .await
            .map_err(|e| {
                DockerConfigError::BuildError(format!("Failed to execute docker start: {}", e))
            })?;

        if output.status.success() {
            info!("Container started successfully: {}", container_id);
            Ok(true)
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to start container {}: {}", container_id, error_output);
            Ok(false)
        }
    }

    /// Remove container
    pub async fn remove_container(&self, container_id: &str, force: bool) -> Result<bool, DockerConfigError> {
        info!("Removing container: {} (force: {})", container_id, force);

        let mut cmd = Command::new("docker");
        cmd.arg("rm");
        if force {
            cmd.arg("-f");
        }
        cmd.arg(container_id);

        let output = cmd.output().await.map_err(|e| {
            DockerConfigError::BuildError(format!("Failed to execute docker rm: {}", e))
        })?;

        if output.status.success() {
            info!("Container removed successfully: {}", container_id);
            Ok(true)
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to remove container {}: {}", container_id, error_output);
            Ok(false)
        }
    }

    /// Get container status
    pub async fn get_container_status(&self, container_id: &str) -> Result<ContainerStatus, DockerConfigError> {
        let output = Command::new("docker")
            .arg("inspect")
            .arg("--format")
            .arg("{{.State.Status}}")
            .arg(container_id)
            .output()
            .await
            .map_err(|e| {
                DockerConfigError::BuildError(format!("Failed to execute docker inspect: {}", e))
            })?;

        if output.status.success() {
            let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let status = match status_str.as_str() {
                "created" => ContainerStatus::Created,
                "running" => ContainerStatus::Running,
                "paused" => ContainerStatus::Paused,
                "restarting" => ContainerStatus::Restarting,
                "removing" => ContainerStatus::Removing,
                "exited" => ContainerStatus::Exited,
                "dead" => ContainerStatus::Dead,
                _ => ContainerStatus::Failed,
            };
            Ok(status)
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            error!("Failed to get container status {}: {}", container_id, error_output);
            Err(DockerConfigError::BuildError(error_output.to_string()))
        }
    }

    /// Get container metrics
    pub async fn get_container_metrics(&self, container_id: &str) -> Result<ContainerMetrics, DockerConfigError> {
        let output = Command::new("docker")
            .arg("stats")
            .arg("--no-stream")
            .arg("--format")
            .arg("{{.CPUPerc}},{{.MemUsage}},{{.NetIO}},{{.BlockIO}},{{.PIDs}}")
            .arg(container_id)
            .output()
            .await
            .map_err(|e| {
                DockerConfigError::BuildError(format!("Failed to execute docker stats: {}", e))
            })?;

        if output.status.success() {
            let stats_str = String::from_utf8_lossy(&output.stdout);
            let stats_trimmed = stats_str.trim();
            let parts: Vec<&str> = stats_trimmed.split(',').collect();

            if parts.len() >= 5 {
                let cpu_percent = parts[0].trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
                let mem_usage = parts[1].trim();
                let (mem_used, mem_limit) = if mem_usage.contains('/') {
                    let mem_parts: Vec<&str> = mem_usage.split('/').collect();
                    (
                        Self::parse_memory(mem_parts[0].trim()),
                        Self::parse_memory(mem_parts[1].trim())
                    )
                } else {
                    (Self::parse_memory(mem_usage), 0)
                };

                let mut metrics = ContainerMetrics::new(uuid::Uuid::new_v4());
                metrics.cpu_usage_percent = cpu_percent;
                metrics.memory_usage_mb = mem_used;
                metrics.memory_limit_mb = mem_limit;
                metrics.calculate_memory_percent();

                Ok(metrics)
            } else {
                error!("Invalid docker stats output format for container {}", container_id);
                Err(DockerConfigError::BuildError("Invalid stats output format".to_string()))
            }
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            error!("Failed to get container metrics {}: {}", container_id, error_output);
            Err(DockerConfigError::BuildError(error_output.to_string()))
        }
    }

    /// List all containers
    pub async fn list_containers(&self, all: bool) -> Result<Vec<HashMap<String, String>>, DockerConfigError> {
        let mut cmd = Command::new("docker");
        cmd.arg("ps");
        if all {
            cmd.arg("-a");
        }
        cmd.arg("--format")
           .arg("{{.ID}}\t{{.Image}}\t{{.Status}}\t{{.Names}}");

        let output = cmd.output().await.map_err(|e| {
            DockerConfigError::BuildError(format!("Failed to execute docker ps: {}", e))
        })?;

        if output.status.success() {
            let containers: Vec<HashMap<String, String>> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    let parts: Vec<&str> = line.split('\t').collect();
                    let mut container = HashMap::new();
                    if parts.len() >= 4 {
                        container.insert("id".to_string(), parts[0].to_string());
                        container.insert("image".to_string(), parts[1].to_string());
                        container.insert("status".to_string(), parts[2].to_string());
                        container.insert("names".to_string(), parts[3].to_string());
                    }
                    container
                })
                .collect();

            Ok(containers)
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            error!("Failed to list containers: {}", error_output);
            Err(DockerConfigError::BuildError(error_output.to_string()))
        }
    }

    /// Execute command in container
    pub async fn exec_command(&self, container_id: &str, command: Vec<String>) -> Result<String, DockerConfigError> {
        let mut cmd = Command::new("docker");
        cmd.arg("exec").arg(container_id);
        cmd.args(command);

        let output = cmd.output().await.map_err(|e| {
            DockerConfigError::BuildError(format!("Failed to execute docker exec: {}", e))
        })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            Err(DockerConfigError::BuildError(error_output.to_string()))
        }
    }

    /// Get container logs
    pub async fn get_container_logs(&self, container_id: &str, tail: Option<i32>) -> Result<String, DockerConfigError> {
        let mut cmd = Command::new("docker");
        cmd.arg("logs");
        if let Some(tail_lines) = tail {
            cmd.arg("--tail").arg(tail_lines.to_string());
        }
        cmd.arg(container_id);

        let output = cmd.output().await.map_err(|e| {
            DockerConfigError::BuildError(format!("Failed to execute docker logs: {}", e))
        })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            Err(DockerConfigError::BuildError(error_output.to_string()))
        }
    }

    /// Parse memory string (e.g., "1.2GiB" -> 1280 MB)
    fn parse_memory(mem_str: &str) -> u64 {
        let mem_bytes = {
            let mem_str = mem_str.trim();
            if mem_str.ends_with("GiB") || mem_str.ends_with("GB") {
                mem_str[..mem_str.len() - 3].parse::<f64>().unwrap_or(0.0) * 1024.0
            } else if mem_str.ends_with("MiB") || mem_str.ends_with("MB") {
                mem_str[..mem_str.len() - 3].parse::<f64>().unwrap_or(0.0)
            } else if mem_str.ends_with("KiB") || mem_str.ends_with("KB") {
                mem_str[..mem_str.len() - 3].parse::<f64>().unwrap_or(0.0) / 1024.0
            } else if mem_str.ends_with('B') {
                mem_str[..mem_str.len() - 1].parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
            } else {
                mem_str.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
            }
        } as u64;
        mem_bytes
    }

    /// Extract image ID from build output
    fn extract_image_id(&self, output: &[u8]) -> Option<String> {
        let output_str = String::from_utf8_lossy(output);
        for line in output_str.lines() {
            if line.contains("Successfully built") {
                if let Some(start) = line.find("Successfully built ") {
                    let id = line[start + "Successfully built ".len()..].trim();
                    return Some(id.to_string());
                }
            }
        }
        None
    }

    /// Ping Docker daemon to check connectivity
    pub async fn ping(&self) -> Result<std::time::Duration, DockerConfigError> {
        let start = std::time::Instant::now();
        let output = Command::new("docker")
            .arg("version")
            .arg("--format")
            .arg("{{.Server.Version}}")
            .output()
            .await
            .map_err(|e| DockerConfigError::BuildError(format!("Failed to ping Docker: {}", e)))?;

        if output.status.success() {
            Ok(start.elapsed())
        } else {
            let error_output = String::from_utf8_lossy(&output.stderr);
            Err(DockerConfigError::BuildError(format!("Docker ping failed: {}", error_output)))
        }
    }
}

/// Result of Docker image build operation
#[derive(Debug, Clone)]
pub struct DockerBuildResult {
    pub success: bool,
    pub image_id: String,
    pub build_time: chrono::DateTime<chrono::Utc>,
    pub logs: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_parsing() {
        assert_eq!(DockerClient::parse_memory("1GiB"), 1024);
        assert_eq!(DockerClient::parse_memory("512MiB"), 512);
        assert_eq!(DockerClient::parse_memory("1.5GiB"), 1536);
        assert_eq!(DockerClient::parse_memory("1024B"), 0); // Less than 1MB
    }

    #[test]
    fn test_docker_client_creation() {
        let client = DockerClient::new("unix:///var/run/docker.sock".to_string());
        assert_eq!(client.host, "unix:///var/run/docker.sock");
        assert_eq!(client.default_timeout.as_secs(), 300);
    }

    #[tokio::test]
    async fn test_docker_client_with_auth() {
        let auth = DockerAuth {
            username: std::env::var("DOCKER_TEST_USERNAME").unwrap_or_else(|_| "test".to_string()),
            password: std::env::var("DOCKER_TEST_PASSWORD").unwrap_or_else(|_| {
                // Default for local testing only - never commit real credentials
                "test_password_for_local_dev_only".to_string()
            }),
            server_address: "docker.io".to_string(),
        };
        let client = DockerClient::with_auth("unix:///var/run/docker.sock".to_string(), auth);
        assert!(client.auth.is_some());
    }
}