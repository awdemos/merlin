use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Tmpfs mount configuration for Docker containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmpfsMount {
    /// Mount point inside the container
    pub mount_point: PathBuf,

    /// Size of the tmpfs (e.g., "100m", "1g")
    pub size: String,

    /// Mount options (e.g., "exec", "noexec", "rw", "ro")
    pub options: String,

    /// Optional mode for the mount point
    pub mode: Option<String>,

    /// Optional UID for the mount point
    pub uid: Option<u32>,

    /// Optional GID for the mount point
    pub gid: Option<u32>,
}

impl Default for TmpfsMount {
    fn default() -> Self {
        Self {
            mount_point: PathBuf::from("/tmp"),
            size: "100m".to_string(),
            options: "exec".to_string(),
            mode: None,
            uid: None,
            gid: None,
        }
    }
}

impl TmpfsMount {
    /// Create a new tmpfs mount
    pub fn new(mount_point: impl AsRef<std::path::Path>, size: String) -> Self {
        Self {
            mount_point: mount_point.as_ref().to_path_buf(),
            size,
            ..Default::default()
        }
    }

    /// Create a tmpfs mount for /tmp directory
    pub fn for_tmp(size: String) -> Self {
        Self::new("/tmp", size)
    }

    /// Create a tmpfs mount for /var/tmp directory
    pub fn for_var_tmp(size: String) -> Self {
        Self::new("/var/tmp", size)
    }

    /// Create a read-only tmpfs mount
    pub fn read_only(mut self) -> Self {
        self.options = if self.options.is_empty() {
            "ro".to_string()
        } else {
            format!("{},ro", self.options)
        };
        self
    }

    /// Create a no-exec tmpfs mount
    pub fn no_exec(mut self) -> Self {
        self.options = if self.options.is_empty() {
            "noexec".to_string()
        } else {
            format!("{},noexec", self.options)
        };
        self
    }

    /// Set the mode for the mount point
    pub fn with_mode(mut self, mode: String) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Set the UID for the mount point
    pub fn with_uid(mut self, uid: u32) -> Self {
        self.uid = Some(uid);
        self
    }

    /// Set the GID for the mount point
    pub fn with_gid(mut self, gid: u32) -> Self {
        self.gid = Some(gid);
        self
    }

    /// Add custom options
    pub fn with_options(mut self, options: String) -> Self {
        if self.options.is_empty() {
            self.options = options;
        } else {
            self.options = format!("{},{}", self.options, options);
        }
        self
    }

    /// Validate the tmpfs mount configuration
    pub fn validate(&self) -> Result<(), TmpfsMountError> {
        if self.mount_point.as_os_str().is_empty() {
            return Err(TmpfsMountError::InvalidMountPoint(
                "Mount point cannot be empty".to_string(),
            ));
        }

        // Validate mount point is an absolute path
        if !self.mount_point.is_absolute() {
            return Err(TmpfsMountError::InvalidMountPoint(
                "Mount point must be an absolute path".to_string(),
            ));
        }

        // Validate size format
        if !self.is_valid_size(&self.size) {
            return Err(TmpfsMountError::InvalidSize(format!(
                "Invalid size format: {}",
                self.size
            )));
        }

        // Validate dangerous mount points
        if self.is_dangerous_mount_point() {
            return Err(TmpfsMountError::SecurityViolation(
                "Mount point is not allowed for security reasons".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if size format is valid
    fn is_valid_size(&self, size: &str) -> bool {
        if size.is_empty() {
            return false;
        }

        // Extract numeric part
        let numeric_part = size.trim_end_matches(|c: char| c.is_alphabetic());
        if numeric_part.is_empty() {
            return false;
        }

        // Parse numeric part
        let num: u64 = numeric_part.parse().unwrap_or(0);
        if num == 0 {
            return false;
        }

        // Check unit
        let unit = size.trim_start_matches(numeric_part);
        matches!(
            unit.to_lowercase().as_str(),
            "k" | "kb" | "m" | "mb" | "g" | "gb" | "t" | "tb"
        )
    }

    /// Check if mount point is potentially dangerous
    fn is_dangerous_mount_point(&self) -> bool {
        let mount_str = self.mount_point.to_string_lossy().to_lowercase();

        // System-critical directories
        mount_str.starts_with("/bin")
            || mount_str.starts_with("/sbin")
            || mount_str.starts_with("/lib")
            || mount_str.starts_with("/usr")
            || mount_str.starts_with("/etc")
            || mount_str.starts_with("/boot")
            || mount_str.starts_with("/dev")
            || mount_str.starts_with("/proc")
            || mount_str.starts_with("/sys")
    }

    /// Get size in bytes
    pub fn size_bytes(&self) -> Result<u64, TmpfsMountError> {
        let size_str = self.size.to_lowercase();

        // Extract numeric part
        let numeric_part = size_str.trim_end_matches(|c: char| c.is_alphabetic());
        let num: u64 = numeric_part.parse().map_err(|_| {
            TmpfsMountError::InvalidSize(format!("Cannot parse size: {}", self.size))
        })?;

        // Convert to bytes based on unit
        let unit = size_str.trim_start_matches(numeric_part);
        let multiplier = match unit {
            "k" | "kb" => 1024,
            "m" | "mb" => 1024 * 1024,
            "g" | "gb" => 1024 * 1024 * 1024,
            "t" | "tb" => 1024 * 1024 * 1024 * 1024,
            _ => {
                return Err(TmpfsMountError::InvalidSize(format!(
                    "Unknown unit: {}",
                    unit
                )))
            }
        };

        Ok(num * multiplier)
    }

    /// Generate Docker tmpfs mount argument
    pub fn docker_arg(&self) -> Result<String, TmpfsMountError> {
        self.validate()?;

        let mut arg = format!("{}:size={}", self.mount_point.display(), self.size);

        if !self.options.is_empty() {
            arg.push(',');
            arg.push_str(&self.options);
        }

        if let Some(ref mode) = self.mode {
            arg.push_str(&format!(",mode={}", mode));
        }

        if let Some(uid) = self.uid {
            arg.push_str(&format!(",uid={}", uid));
        }

        if let Some(gid) = self.gid {
            arg.push_str(&format!(",gid={}", gid));
        }

        Ok(arg)
    }

    /// Check if this is a standard system tmpfs mount
    pub fn is_system_mount(&self) -> bool {
        let mount_str = self.mount_point.to_string_lossy().to_lowercase();
        mount_str == "/tmp" || mount_str == "/var/tmp" || mount_str == "/run"
    }

    /// Calculate security score for this tmpfs mount (0-100)
    pub fn security_score(&self) -> u8 {
        let mut score = 100;

        // Penalize large tmpfs mounts
        if let Ok(size_bytes) = self.size_bytes() {
            if size_bytes > 1024 * 1024 * 1024 {
                // > 1GB
                score -= 20;
            } else if size_bytes > 512 * 1024 * 1024 {
                // > 512MB
                score -= 10;
            }
        }

        // Reward noexec option for security
        if self.options.contains("noexec") {
            score += 10;
        }

        // Reward read-only option
        if self.options.contains("ro") {
            score += 5;
        }

        // Penalize exec option on non-tmp directories
        if self.options.contains("exec") && !self.mount_point.to_string_lossy().contains("tmp") {
            score -= 10;
        }

        // Reward standard system mounts
        if self.is_system_mount() {
            score += 5;
        }

        score.max(0).min(100)
    }
}

impl fmt::Display for TmpfsMount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TmpfsMount(mount_point={}, size={}, options={}, security_score={})",
            self.mount_point.display(),
            self.size,
            self.options,
            self.security_score()
        )
    }
}

/// Errors related to tmpfs mounts
#[derive(Debug, thiserror::Error)]
pub enum TmpfsMountError {
    #[error("Invalid mount point: {0}")]
    InvalidMountPoint(String),

    #[error("Invalid size: {0}")]
    InvalidSize(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Invalid options: {0}")]
    InvalidOptions(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tmpfs() {
        let tmpfs = TmpfsMount::default();
        assert_eq!(tmpfs.mount_point, PathBuf::from("/tmp"));
        assert_eq!(tmpfs.size, "100m");
        assert_eq!(tmpfs.options, "exec");
    }

    #[test]
    fn test_new_tmpfs() {
        let tmpfs = TmpfsMount::new("/custom/tmp", "50m".to_string());
        assert_eq!(tmpfs.mount_point, PathBuf::from("/custom/tmp"));
        assert_eq!(tmpfs.size, "50m");
    }

    #[test]
    fn test_for_tmp() {
        let tmpfs = TmpfsMount::for_tmp("200m".to_string());
        assert_eq!(tmpfs.mount_point, PathBuf::from("/tmp"));
        assert_eq!(tmpfs.size, "200m");
    }

    #[test]
    fn test_for_var_tmp() {
        let tmpfs = TmpfsMount::for_var_tmp("150m".to_string());
        assert_eq!(tmpfs.mount_point, PathBuf::from("/var/tmp"));
        assert_eq!(tmpfs.size, "150m");
    }

    #[test]
    fn test_read_only() {
        let tmpfs = TmpfsMount::for_tmp("100m".to_string()).read_only();
        assert!(tmpfs.options.contains("ro"));
    }

    #[test]
    fn test_no_exec() {
        let tmpfs = TmpfsMount::for_tmp("100m".to_string()).no_exec();
        assert!(tmpfs.options.contains("noexec"));
    }

    #[test]
    fn test_with_mode() {
        let tmpfs = TmpfsMount::for_tmp("100m".to_string()).with_mode("1777".to_string());
        assert_eq!(tmpfs.mode, Some("1777".to_string()));
    }

    #[test]
    fn test_validate_valid() {
        let tmpfs = TmpfsMount::for_tmp("100m".to_string());
        assert!(tmpfs.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_mount_point() {
        let mut tmpfs = TmpfsMount::for_tmp("100m".to_string());
        tmpfs.mount_point = PathBuf::new();
        assert!(matches!(
            tmpfs.validate(),
            Err(TmpfsMountError::InvalidMountPoint(_))
        ));
    }

    #[test]
    fn test_validate_relative_path() {
        let tmpfs = TmpfsMount::new("relative/path", "100m".to_string());
        assert!(matches!(
            tmpfs.validate(),
            Err(TmpfsMountError::InvalidMountPoint(_))
        ));
    }

    #[test]
    fn test_validate_invalid_size() {
        let tmpfs = TmpfsMount::for_tmp("invalid".to_string());
        assert!(matches!(
            tmpfs.validate(),
            Err(TmpfsMountError::InvalidSize(_))
        ));
    }

    #[test]
    fn test_validate_dangerous_mount_point() {
        let tmpfs = TmpfsMount::new("/bin", "100m".to_string());
        assert!(matches!(
            tmpfs.validate(),
            Err(TmpfsMountError::SecurityViolation(_))
        ));
    }

    #[test]
    fn test_size_bytes() {
        let tmpfs = TmpfsMount::for_tmp("1m".to_string());
        assert_eq!(tmpfs.size_bytes().unwrap(), 1024 * 1024);

        let tmpfs = TmpfsMount::for_tmp("1g".to_string());
        assert_eq!(tmpfs.size_bytes().unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_docker_arg() {
        let tmpfs = TmpfsMount::for_tmp("100m".to_string());
        assert_eq!(tmpfs.docker_arg().unwrap(), "/tmp:size=100m,exec");

        let tmpfs = TmpfsMount::for_tmp("100m".to_string())
            .read_only()
            .with_mode("1777".to_string());
        let arg = tmpfs.docker_arg().unwrap();
        assert!(arg.contains("ro"));
        assert!(arg.contains("mode=1777"));
    }

    #[test]
    fn test_is_system_mount() {
        assert!(TmpfsMount::for_tmp("100m".to_string()).is_system_mount());
        assert!(TmpfsMount::for_var_tmp("100m".to_string()).is_system_mount());
        assert!(!TmpfsMount::new("/custom", "100m".to_string()).is_system_mount());
    }

    #[test]
    fn test_security_score() {
        let secure = TmpfsMount::for_tmp("50m".to_string()).no_exec().read_only();
        let insecure =
            TmpfsMount::new("/custom", "2g".to_string()).with_options("exec".to_string());

        assert!(secure.security_score() > insecure.security_score());
    }

    #[test]
    fn test_display() {
        let tmpfs = TmpfsMount::for_tmp("100m".to_string());
        let display = format!("{}", tmpfs);
        assert!(display.contains("mount_point=/tmp"));
        assert!(display.contains("size=100m"));
        assert!(display.contains("options=exec"));
    }
}
