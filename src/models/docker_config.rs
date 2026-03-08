use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Docker container configuration for building and deploying Merlin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContainerConfig {
    /// Name of the Docker image to build
    pub image_name: String,

    /// Path to the Dockerfile
    pub dockerfile_path: String,

    /// Build arguments for Docker build
    pub build_args: Vec<(String, String)>,

    /// Tags to apply to the built image
    pub tags: Vec<String>,

    /// Security context for the container
    pub security_context: Option<serde_json::Value>,

    /// Resource limits for the container
    pub resource_limits: Option<ResourceLimits>,

    /// Network configuration
    pub network_config: Option<NetworkConfig>,

    /// Health check configuration
    pub health_check: Option<HealthCheckConfig>,

    /// Environment variables
    pub environment: HashMap<String, String>,

    /// Volume mounts
    pub volumes: Vec<VolumeMount>,

    /// Tmpfs mounts
    pub tmpfs_mounts: Vec<TmpfsMount>,

    /// Unique identifier for this configuration
    pub id: Uuid,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for DockerContainerConfig {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            image_name: String::new(),
            dockerfile_path: String::new(),
            build_args: Vec::new(),
            tags: Vec::new(),
            security_context: None,
            resource_limits: None,
            network_config: None,
            health_check: None,
            environment: HashMap::new(),
            volumes: Vec::new(),
            tmpfs_mounts: Vec::new(),
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl DockerContainerConfig {
    /// Create a new Docker container configuration
    pub fn new(image_name: String, dockerfile_path: String) -> Self {
        let mut config = Self::default();
        config.image_name = image_name;
        config.dockerfile_path = dockerfile_path;
        config
    }

    /// Add a build argument
    pub fn add_build_arg(&mut self, key: String, value: String) {
        self.build_args.push((key, value));
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
    }

    /// Add an environment variable
    pub fn add_environment(&mut self, key: String, value: String) {
        self.environment.insert(key, value);
    }

    /// Add a volume mount
    pub fn add_volume(&mut self, volume: VolumeMount) {
        self.volumes.push(volume);
    }

    /// Add a tmpfs mount
    pub fn add_tmpfs(&mut self, tmpfs: TmpfsMount) {
        self.tmpfs_mounts.push(tmpfs);
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), DockerConfigError> {
        if self.image_name.is_empty() {
            return Err(DockerConfigError::InvalidImageName("Image name cannot be empty".to_string()));
        }

        if self.dockerfile_path.is_empty() {
            return Err(DockerConfigError::InvalidDockerfile("Dockerfile path cannot be empty".to_string()));
        }

        // Validate image name format
        if !self.image_name.chars().all(|c| c.is_alphanumeric() || c == ':' || c == '-' || c == '_' || c == '.') {
            return Err(DockerConfigError::InvalidImageName("Invalid image name format".to_string()));
        }

        // Validate Dockerfile path
        if !std::path::Path::new(&self.dockerfile_path).exists() {
            return Err(DockerConfigError::DockerfileNotFound(format!("Dockerfile not found: {}", self.dockerfile_path)));
        }

        // Validate resource limits if present
        if let Some(ref limits) = self.resource_limits {
            limits.validate()?;
        }

        // Validate security context if present
        if let Some(ref security) = self.security_context {
            self.validate_security_context(security)?;
        }

        Ok(())
    }

    /// Validate security context
    fn validate_security_context(&self, security: &serde_json::Value) -> Result<(), DockerConfigError> {
        if let Some(user) = security.get("user") {
            if let Some(user_str) = user.as_str() {
                if user_str == "root" || user_str == "0" {
                    return Err(DockerConfigError::SecurityViolation(
                        "Container should not run as root user".to_string()
                    ));
                }
            }
        }

        if let Some(read_only) = security.get("read_only") {
            if let Some(ro) = read_only.as_bool() {
                if !ro {
                    return Err(DockerConfigError::SecurityViolation(
                        "Container should have read-only filesystem".to_string()
                    ));
                }
            }
        }

        Ok(())
    }

    /// Generate Docker build command
    pub fn build_command(&self) -> Vec<String> {
        let mut cmd = vec!["build".to_string(), "-t".to_string(), self.image_name.clone()];

        cmd.push("-f".to_string());
        cmd.push(self.dockerfile_path.clone());

        // Add build arguments
        for (key, value) in &self.build_args {
            cmd.push("--build-arg".to_string());
            cmd.push(format!("{}={}", key, value));
        }

        cmd.push(".".to_string());
        cmd
    }

    /// Generate Docker run command
    pub fn run_command(&self, container_name: Option<String>) -> Vec<String> {
        let mut cmd = vec!["run".to_string()];

        if let Some(name) = container_name {
            cmd.push("--name".to_string());
            cmd.push(name);
        }

        // Add security options
        if let Some(ref security) = self.security_context {
            if let Some(read_only) = security.get("read_only").and_then(|v| v.as_bool()) {
                if read_only {
                    cmd.push("--read-only".to_string());
                }
            }

            if let Some(user) = security.get("user").and_then(|v| v.as_str()) {
                cmd.push("--user".to_string());
                cmd.push(user.to_string());
            }
        }

        // Add resource limits
        if let Some(ref limits) = self.resource_limits {
            if limits.memory_mb > 0 {
                cmd.push("--memory".to_string());
                cmd.push(format!("{}m", limits.memory_mb));
            }

            if limits.cpu_shares > 0 {
                cmd.push("--cpus".to_string());
                cmd.push(format!("{}", limits.cpu_shares));
            }

            if limits.pids_limit > 0 {
                cmd.push("--pids-limit".to_string());
                cmd.push(format!("{}", limits.pids_limit));
            }
        }

        // Add environment variables
        for (key, value) in &self.environment {
            cmd.push("-e".to_string());
            cmd.push(format!("{}={}", key, value));
        }

        // Add volumes
        for volume in &self.volumes {
            cmd.push("-v".to_string());
            cmd.push(format!("{}:{}", volume.source, volume.destination));
        }

        // Add tmpfs mounts
        for tmpfs in &self.tmpfs_mounts {
            cmd.push("--tmpfs".to_string());
            cmd.push(format!("{}:size={},{}", tmpfs.mount_point, tmpfs.size, tmpfs.options));
        }

        // Add health check if present
        if let Some(ref health) = self.health_check {
            cmd.push("--health-cmd".to_string());
            cmd.push(health.command.clone());

            cmd.push("--health-interval".to_string());
            cmd.push(format!("{}s", health.interval_seconds));

            cmd.push("--health-timeout".to_string());
            cmd.push(format!("{}s", health.timeout_seconds));

            cmd.push("--health-retries".to_string());
            cmd.push(format!("{}", health.retries));
        }

        cmd.push(self.image_name.clone());
        cmd
    }
}

/// Errors related to Docker configuration
#[derive(Debug, thiserror::Error)]
pub enum DockerConfigError {
    #[error("Invalid image name: {0}")]
    InvalidImageName(String),

    #[error("Invalid Dockerfile: {0}")]
    InvalidDockerfile(String),

    #[error("Dockerfile not found: {0}")]
    DockerfileNotFound(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Resource limits error: {0}")]
    ResourceLimitsError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl From<ResourceLimitsError> for DockerConfigError {
    fn from(err: ResourceLimitsError) -> Self {
        DockerConfigError::ResourceLimitsError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DockerContainerConfig::default();
        assert!(config.image_name.is_empty());
        assert!(config.dockerfile_path.is_empty());
        assert!(config.build_args.is_empty());
        assert!(config.tags.is_empty());
    }

    #[test]
    fn test_new_config() {
        let config = DockerContainerConfig::new("test:latest".to_string(), "Dockerfile".to_string());
        assert_eq!(config.image_name, "test:latest");
        assert_eq!(config.dockerfile_path, "Dockerfile");
    }

    #[test]
    fn test_add_build_arg() {
        let mut config = DockerContainerConfig::default();
        config.add_build_arg("RUST_ENV".to_string(), "production".to_string());
        assert_eq!(config.build_args.len(), 1);
        assert_eq!(config.build_args[0], ("RUST_ENV".to_string(), "production".to_string()));
    }

    #[test]
    fn test_validate_empty_image_name() {
        let config = DockerContainerConfig::default();
        let result = config.validate();
        assert!(matches!(result, Err(DockerConfigError::InvalidImageName(_))));
    }

    #[test]
    fn test_validate_root_user() {
        let mut config = DockerContainerConfig::new("test:latest".to_string(), "Dockerfile".to_string());
        config.security_context = Some(serde_json::json!({"user": "root"}));

        let result = config.validate();
        assert!(matches!(result, Err(DockerConfigError::SecurityViolation(_))));
    }

    #[test]
    fn test_build_command() {
        let mut config = DockerContainerConfig::new("test:latest".to_string(), "Dockerfile".to_string());
        config.add_build_arg("RUST_ENV".to_string(), "production".to_string());

        let cmd = config.build_command();
        assert_eq!(cmd[0], "build");
        assert_eq!(cmd[1], "-t");
        assert_eq!(cmd[2], "test:latest");
        assert_eq!(cmd[3], "-f");
        assert_eq!(cmd[4], "Dockerfile");
        assert_eq!(cmd[5], "--build-arg");
        assert_eq!(cmd[6], "RUST_ENV=production");
        assert_eq!(cmd[7], ".");
    }

    #[test]
    fn test_run_command() {
        let mut config = DockerContainerConfig::new("test:latest".to_string(), "Dockerfile".to_string());
        config.add_environment("RUST_ENV".to_string(), "production".to_string());

        let cmd = config.run_command(Some("test-container".to_string()));
        assert_eq!(cmd[0], "run");
        assert_eq!(cmd[1], "--name");
        assert_eq!(cmd[2], "test-container");
        assert_eq!(cmd[3], "-e");
        assert_eq!(cmd[4], "RUST_ENV=production");
        assert_eq!(cmd[5], "test:latest");
    }
}