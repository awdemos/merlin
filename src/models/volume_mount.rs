use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Volume mount configuration for Docker containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Source path on the host
    pub source: PathBuf,

    /// Destination path in the container
    pub destination: PathBuf,

    /// Mount mode: "ro" (read-only) or "rw" (read-write)
    pub mode: String,

    /// Volume name (for named volumes)
    pub name: Option<String>,

    /// Whether this is a bind mount (true) or named volume (false)
    pub is_bind_mount: bool,

    /// Propagation mode for bind mounts
    pub propagation: Option<String>,

    /// SELinux label
    pub selinux_label: Option<String>,
}

impl Default for VolumeMount {
    fn default() -> Self {
        Self {
            source: PathBuf::new(),
            destination: PathBuf::new(),
            mode: "ro".to_string(),
            name: None,
            is_bind_mount: true,
            propagation: None,
            selinux_label: None,
        }
    }
}

impl VolumeMount {
    /// Create a new bind mount
    pub fn bind_mount(source: impl AsRef<std::path::Path>, destination: impl AsRef<std::path::Path>) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            destination: destination.as_ref().to_path_buf(),
            is_bind_mount: true,
            ..Default::default()
        }
    }

    /// Create a new named volume
    pub fn named_volume(name: String, destination: impl AsRef<std::path::Path>) -> Self {
        Self {
            name: Some(name),
            destination: destination.as_ref().to_path_buf(),
            is_bind_mount: false,
            ..Default::default()
        }
    }

    /// Make mount read-only
    pub fn read_only(mut self) -> Self {
        self.mode = "ro".to_string();
        self
    }

    /// Make mount read-write
    pub fn read_write(mut self) -> Self {
        self.mode = "rw".to_string();
        self
    }

    /// Set propagation mode for bind mounts
    pub fn with_propagation(mut self, propagation: String) -> Self {
        self.propagation = Some(propagation);
        self
    }

    /// Set SELinux label
    pub fn with_selinux_label(mut self, label: String) -> Self {
        self.selinux_label = Some(label);
        self
    }

    /// Validate the volume mount configuration
    pub fn validate(&self) -> Result<(), VolumeMountError> {
        if self.destination.as_os_str().is_empty() {
            return Err(VolumeMountError::InvalidDestination("Destination path cannot be empty".to_string()));
        }

        // Validate destination is an absolute path
        if !self.destination.is_absolute() {
            return Err(VolumeMountError::InvalidDestination("Destination path must be absolute".to_string()));
        }

        if self.is_bind_mount {
            if self.source.as_os_str().is_empty() {
                return Err(VolumeMountError::InvalidSource("Source path cannot be empty for bind mounts".to_string()));
            }

            // Validate source exists (for bind mounts)
            if !self.source.exists() && !self.source.to_string_lossy().starts_with("npipe://") {
                return Err(VolumeMountError::SourceNotFound(format!("Source path does not exist: {}", self.source.display())));
            }

            // Validate dangerous source paths
            if self.is_dangerous_source_path() {
                return Err(VolumeMountError::SecurityViolation(
                    "Source path is not allowed for security reasons".to_string()
                ));
            }
        } else {
            if self.name.is_none() || self.name.as_ref().unwrap().is_empty() {
                return Err(VolumeMountError::InvalidName("Volume name cannot be empty for named volumes".to_string()));
            }
        }

        // Validate mode
        if self.mode != "ro" && self.mode != "rw" {
            return Err(VolumeMountError::InvalidMode(format!("Invalid mount mode: {}", self.mode)));
        }

        // Validate destination is not a dangerous path
        if self.is_dangerous_destination_path() {
            return Err(VolumeMountError::SecurityViolation(
                "Destination path is not allowed for security reasons".to_string()
            ));
        }

        Ok(())
    }

    /// Check if source path is potentially dangerous
    fn is_dangerous_source_path(&self) -> bool {
        if !self.is_bind_mount {
            return false;
        }

        let source_str = self.source.to_string_lossy().to_lowercase();

        // System-critical directories
        source_str.starts_with("/bin") ||
        source_str.starts_with("/sbin") ||
        source_str.starts_with("/lib") ||
        source_str.starts_with("/usr") ||
        source_str.starts_with("/etc") ||
        source_str.starts_with("/boot") ||
        source_str.starts_with("/dev") ||
        source_str.starts_with("/proc") ||
        source_str.starts_with("/sys") ||
        source_str.contains("/docker") ||
        source_str.contains("/var/lib/docker")
    }

    /// Check if destination path is potentially dangerous
    fn is_dangerous_destination_path(&self) -> bool {
        let dest_str = self.destination.to_string_lossy().to_lowercase();

        // System-critical directories
        dest_str.starts_with("/bin") ||
        dest_str.starts_with("/sbin") ||
        dest_str.starts_with("/lib") ||
        dest_str.starts_with("/usr") ||
        dest_str.starts_with("/etc") ||
        dest_str.starts_with("/boot") ||
        dest_str.starts_with("/dev") ||
        dest_str.starts_with("/proc") ||
        dest_str.starts_with("/sys")
    }

    /// Generate Docker volume mount argument
    pub fn docker_arg(&self) -> Result<String, VolumeMountError> {
        self.validate()?;

        if self.is_bind_mount {
            let mut arg = format!("{}:{}", self.source.display(), self.destination.display());

            // Add mode
            if !self.mode.is_empty() {
                arg.push(':');
                arg.push_str(&self.mode);
            }

            // Add propagation
            if let Some(ref propagation) = self.propagation {
                arg.push(',');
                arg.push_str(propagation);
            }

            // Add SELinux label
            if let Some(ref label) = self.selinux_label {
                arg.push(',');
                arg.push_str(label);
            }

            Ok(arg)
        } else {
            let mut arg = format!("{}:{}", self.name.as_ref().unwrap(), self.destination.display());

            // Add mode
            if !self.mode.is_empty() {
                arg.push(':');
                arg.push_str(&self.mode);
            }

            Ok(arg)
        }
    }

    /// Check if this mount is read-only
    pub fn is_read_only(&self) -> bool {
        self.mode == "ro"
    }

    /// Check if this mount is read-write
    pub fn is_read_write(&self) -> bool {
        self.mode == "rw"
    }

    /// Check if this is a configuration mount
    pub fn is_config_mount(&self) -> bool {
        let dest_str = self.destination.to_string_lossy().to_lowercase();
        dest_str.contains("config") || dest_str.contains("conf")
    }

    /// Check if this is a log mount
    pub fn is_log_mount(&self) -> bool {
        let dest_str = self.destination.to_string_lossy().to_lowercase();
        dest_str.contains("log") || dest_str.contains("/var/log")
    }

    /// Check if this is a data mount
    pub fn is_data_mount(&self) -> bool {
        let dest_str = self.destination.to_string_lossy().to_lowercase();
        dest_str.contains("data") || dest_str.contains("/var/lib") || dest_str.contains("/app/data")
    }

    /// Calculate security score for this volume mount (0-100)
    pub fn security_score(&self) -> u8 {
        let mut score = 100;

        // Reward read-only mounts
        if self.is_read_only() {
            score += 20;
        }

        // Penalize read-write mounts to system directories
        if self.is_read_write() && self.is_dangerous_destination_path() {
            score -= 50;
        }

        // Penalize bind mounts from host system directories
        if self.is_bind_mount && self.is_dangerous_source_path() {
            score -= 30;
        }

        // Reward configuration mounts (usually read-only)
        if self.is_config_mount() && self.is_read_only() {
            score += 10;
        }

        // Reward log mounts if properly handled
        if self.is_log_mount() {
            if self.is_read_only() {
                score -= 10; // Logs should be writable
            } else {
                score += 5;
            }
        }

        // Reward data mounts with proper isolation
        if self.is_data_mount() && self.is_bind_mount {
            score += 5;
        }

        // Reward named volumes over bind mounts
        if !self.is_bind_mount {
            score += 10;
        }

        score.max(0).min(100)
    }
}

impl fmt::Display for VolumeMount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_bind_mount {
            write!(
                f,
                "VolumeMount(bind: {} -> {}, mode={}, security_score={})",
                self.source.display(),
                self.destination.display(),
                self.mode,
                self.security_score()
            )
        } else {
            write!(
                f,
                "VolumeMount(named: {} -> {}, mode={}, security_score={})",
                self.name.as_ref().unwrap_or(&"unnamed".to_string()),
                self.destination.display(),
                self.mode,
                self.security_score()
            )
        }
    }
}

/// Errors related to volume mounts
#[derive(Debug, thiserror::Error)]
pub enum VolumeMountError {
    #[error("Invalid source path: {0}")]
    InvalidSource(String),

    #[error("Invalid destination path: {0}")]
    InvalidDestination(String),

    #[error("Source path not found: {0}")]
    SourceNotFound(String),

    #[error("Invalid mount mode: {0}")]
    InvalidMode(String),

    #[error("Invalid volume name: {0}")]
    InvalidName(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_volume_mount() {
        let volume = VolumeMount::default();
        assert!(volume.source.as_os_str().is_empty());
        assert!(volume.destination.as_os_str().is_empty());
        assert_eq!(volume.mode, "ro");
        assert!(volume.is_bind_mount);
    }

    #[test]
    fn test_bind_mount() {
        let volume = VolumeMount::bind_mount("/host/path", "/container/path");
        assert_eq!(volume.source, PathBuf::from("/host/path"));
        assert_eq!(volume.destination, PathBuf::from("/container/path"));
        assert!(volume.is_bind_mount);
    }

    #[test]
    fn test_named_volume() {
        let volume = VolumeMount::named_volume("my_volume".to_string(), "/container/path");
        assert_eq!(volume.name, Some("my_volume".to_string()));
        assert_eq!(volume.destination, PathBuf::from("/container/path"));
        assert!(!volume.is_bind_mount);
    }

    #[test]
    fn test_read_only() {
        let volume = VolumeMount::bind_mount("/host/path", "/container/path").read_only();
        assert_eq!(volume.mode, "ro");
        assert!(volume.is_read_only());
    }

    #[test]
    fn test_read_write() {
        let volume = VolumeMount::bind_mount("/host/path", "/container/path").read_write();
        assert_eq!(volume.mode, "rw");
        assert!(volume.is_read_write());
    }

    #[test]
    fn test_validate_valid_bind_mount() {
        let temp_dir = tempdir().unwrap();
        let volume = VolumeMount::bind_mount(temp_dir.path(), "/container/path");
        assert!(volume.validate().is_ok());
    }

    #[test]
    fn test_validate_nonexistent_source() {
        let volume = VolumeMount::bind_mount("/nonexistent/path", "/container/path");
        assert!(matches!(volume.validate(), Err(VolumeMountError::SourceNotFound(_))));
    }

    #[test]
    fn test_validate_relative_destination() {
        let temp_dir = tempdir().unwrap();
        let volume = VolumeMount::bind_mount(temp_dir.path(), "relative/path");
        assert!(matches!(volume.validate(), Err(VolumeMountError::InvalidDestination(_))));
    }

    #[test]
    fn test_validate_dangerous_source() {
        let volume = VolumeMount::bind_mount("/etc", "/container/etc");
        assert!(matches!(volume.validate(), Err(VolumeMountError::SecurityViolation(_))));
    }

    #[test]
    fn test_validate_dangerous_destination() {
        let temp_dir = tempdir().unwrap();
        let volume = VolumeMount::bind_mount(temp_dir.path(), "/bin");
        assert!(matches!(volume.validate(), Err(VolumeMountError::SecurityViolation(_))));
    }

    #[test]
    fn test_docker_arg_bind_mount() {
        let temp_dir = tempdir().unwrap();
        let volume = VolumeMount::bind_mount(temp_dir.path(), "/container/path").read_only();
        let arg = volume.docker_arg().unwrap();
        assert!(arg.contains(&temp_dir.path().to_string_lossy()));
        assert!(arg.contains("/container/path"));
        assert!(arg.contains("ro"));
    }

    #[test]
    fn test_docker_arg_named_volume() {
        let volume = VolumeMount::named_volume("my_volume".to_string(), "/container/path").read_write();
        let arg = volume.docker_arg().unwrap();
        assert_eq!(arg, "my_volume:/container/path:rw");
    }

    #[test]
    fn test_config_mount_detection() {
        let volume = VolumeMount::bind_mount("/host/config", "/app/config");
        assert!(volume.is_config_mount());
    }

    #[test]
    fn test_log_mount_detection() {
        let volume = VolumeMount::bind_mount("/host/logs", "/var/log/app");
        assert!(volume.is_log_mount());
    }

    #[test]
    fn test_data_mount_detection() {
        let volume = VolumeMount::bind_mount("/host/data", "/app/data");
        assert!(volume.is_data_mount());
    }

    #[test]
    fn test_security_score() {
        let safe = VolumeMount::bind_mount("/host/config", "/app/config").read_only();
        let unsafe_volume = VolumeMount::bind_mount("/etc", "/container/etc").read_write();

        assert!(safe.security_score() > unsafe_volume.security_score());
    }

    #[test]
    fn test_display() {
        let volume = VolumeMount::bind_mount("/host/path", "/container/path").read_only();
        let display = format!("{}", volume);
        assert!(display.contains("bind: /host/path -> /container/path"));
        assert!(display.contains("mode=ro"));
    }
}