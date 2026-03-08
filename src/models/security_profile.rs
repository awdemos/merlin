use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// Security profile for Docker containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProfile {
    /// User to run the container as (UID:GID or username)
    pub user: String,

    /// Whether to use read-only filesystem
    pub read_only: bool,

    /// Capabilities to drop
    pub capabilities_drop: HashSet<String>,

    /// Capabilities to add
    pub capabilities_add: HashSet<String>,

    /// Security options
    pub security_options: Vec<String>,

    /// No new privileges flag
    pub no_new_privileges: bool,

    /// Process namespace
    pub pid_namespace: Option<String>,

    /// Network namespace
    pub network_namespace: Option<String>,

    /// IPC namespace
    pub ipc_namespace: Option<String>,

    /// UTS namespace
    pub uts_namespace: Option<String>,

    /// User namespace
    pub user_namespace: Option<String>,

    /// Seccomp profile
    pub seccomp_profile: Option<String>,

    /// AppArmor profile
    pub apparmor_profile: Option<String>,

    /// Read-only paths (for read-write containers)
    pub read_only_paths: Vec<String>,

    /// Read-write paths (for read-only containers)
    pub read_write_paths: Vec<String>,

    /// Masked paths (paths that should be inaccessible)
    pub masked_paths: Vec<String>,

    /// Security context level
    pub security_level: SecurityLevel,

    /// Compliance standards to enforce
    pub compliance_standards: HashSet<String>,
}

impl Default for SecurityProfile {
    fn default() -> Self {
        let mut capabilities_drop = HashSet::new();
        capabilities_drop.insert("ALL".to_string());

        let mut capabilities_add = HashSet::new();
        capabilities_add.insert("CHOWN".to_string());
        capabilities_add.insert("SETGID".to_string());
        capabilities_add.insert("SETUID".to_string());

        let mut security_options = Vec::new();
        security_options.push("no-new-privileges".to_string());

        let mut compliance_standards = HashSet::new();
        compliance_standards.insert("CIS_Docker_Benchmark".to_string());
        compliance_standards.insert("NIST_800_190".to_string());

        Self {
            user: "1000:1000".to_string(),
            read_only: true,
            capabilities_drop,
            capabilities_add,
            security_options,
            no_new_privileges: true,
            pid_namespace: None,
            network_namespace: None,
            ipc_namespace: None,
            uts_namespace: None,
            user_namespace: None,
            seccomp_profile: None,
            apparmor_profile: None,
            read_only_paths: Vec::new(),
            read_write_paths: Vec::new(),
            masked_paths: vec!["/proc/kcore".to_string(), "/proc/latency_stats".to_string()],
            security_level: SecurityLevel::High,
            compliance_standards,
        }
    }
}

impl SecurityProfile {
    /// Create a minimal security profile
    pub fn minimal() -> Self {
        Self {
            user: "1000:1000".to_string(),
            read_only: false,
            capabilities_drop: HashSet::new(),
            capabilities_add: HashSet::new(),
            security_options: Vec::new(),
            no_new_privileges: false,
            security_level: SecurityLevel::Minimal,
            ..Default::default()
        }
    }

    /// Create a standard security profile
    pub fn standard() -> Self {
        let mut capabilities_drop = HashSet::new();
        capabilities_drop.insert("NET_RAW".to_string());
        capabilities_drop.insert("NET_ADMIN".to_string());

        let mut capabilities_add = HashSet::new();
        capabilities_add.insert("CHOWN".to_string());
        capabilities_add.insert("SETGID".to_string());
        capabilities_add.insert("SETUID".to_string());

        let mut security_options = Vec::new();
        security_options.push("no-new-privileges".to_string());

        Self {
            user: "1000:1000".to_string(),
            read_only: false,
            capabilities_drop,
            capabilities_add,
            security_options,
            no_new_privileges: true,
            security_level: SecurityLevel::Standard,
            ..Default::default()
        }
    }

    /// Create a high security profile
    pub fn high() -> Self {
        Self::default()
    }

    /// Create a maximum security profile
    pub fn maximum() -> Self {
        let mut capabilities_drop = HashSet::new();
        capabilities_drop.insert("ALL".to_string());

        let mut security_options = Vec::new();
        security_options.push("no-new-privileges".to_string());

        let mut masked_paths = vec![
            "/proc/kcore".to_string(),
            "/proc/latency_stats".to_string(),
            "/proc/timer_list".to_string(),
            "/proc/sched_debug".to_string(),
            "/sys/firmware".to_string(),
        ];

        let mut read_write_paths = vec![
            "/tmp".to_string(),
            "/var/tmp".to_string(),
            "/run".to_string(),
        ];

        let mut compliance_standards = HashSet::new();
        compliance_standards.insert("CIS_Docker_Benchmark".to_string());
        compliance_standards.insert("NIST_800_190".to_string());
        compliance_standards.insert("PCI_DSS".to_string());
        compliance_standards.insert("SOC2".to_string());

        Self {
            user: "1000:1000".to_string(),
            read_only: true,
            capabilities_drop,
            capabilities_add: HashSet::new(),
            security_options,
            no_new_privileges: true,
            pid_namespace: Some("host".to_string()),
            network_namespace: None,
            ipc_namespace: Some("none".to_string()),
            uts_namespace: Some("host".to_string()),
            user_namespace: None,
            seccomp_profile: Some("docker/default".to_string()),
            apparmor_profile: Some("docker-default".to_string()),
            read_only_paths: Vec::new(),
            read_write_paths,
            masked_paths,
            security_level: SecurityLevel::Maximum,
            compliance_standards,
        }
    }

    /// Set user
    pub fn with_user(mut self, user: String) -> Self {
        self.user = user;
        self
    }

    /// Make filesystem read-only
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Make filesystem read-write
    pub fn read_write(mut self) -> Self {
        self.read_only = false;
        self
    }

    /// Add capability to drop
    pub fn drop_capability(mut self, capability: String) -> Self {
        self.capabilities_drop.insert(capability);
        self
    }

    /// Add capability to add
    pub fn add_capability(mut self, capability: String) -> Self {
        self.capabilities_add.insert(capability);
        self
    }

    /// Add security option
    pub fn add_security_option(mut self, option: String) -> Self {
        self.security_options.push(option);
        self
    }

    /// Enable/disable no new privileges
    pub fn with_no_new_privileges(mut self, enabled: bool) -> Self {
        self.no_new_privileges = enabled;
        self
    }

    /// Set PID namespace
    pub fn with_pid_namespace(mut self, namespace: String) -> Self {
        self.pid_namespace = Some(namespace);
        self
    }

    /// Set seccomp profile
    pub fn with_seccomp_profile(mut self, profile: String) -> Self {
        self.seccomp_profile = Some(profile);
        self
    }

    /// Set AppArmor profile
    pub fn with_apparmor_profile(mut self, profile: String) -> Self {
        self.apparmor_profile = Some(profile);
        self
    }

    /// Add read-only path
    pub fn add_read_only_path(mut self, path: String) -> Self {
        self.read_only_paths.push(path);
        self
    }

    /// Add read-write path
    pub fn add_read_write_path(mut self, path: String) -> Self {
        self.read_write_paths.push(path);
        self
    }

    /// Add masked path
    pub fn add_masked_path(mut self, path: String) -> Self {
        self.masked_paths.push(path);
        self
    }

    /// Add compliance standard
    pub fn add_compliance_standard(mut self, standard: String) -> Self {
        self.compliance_standards.insert(standard);
        self
    }

    /// Set security level
    pub fn with_security_level(mut self, level: SecurityLevel) -> Self {
        self.security_level = level;
        self
    }

    /// Validate the security profile
    pub fn validate(&self) -> Result<(), SecurityProfileError> {
        // Validate user format
        if self.user.is_empty() {
            return Err(SecurityProfileError::InvalidUser("User cannot be empty".to_string()));
        }

        // Check for root user
        if self.user == "root" || self.user == "0" || self.user == "0:0" {
            return Err(SecurityProfileError::SecurityViolation(
                "Container should not run as root user".to_string()
            ));
        }

        // Validate capabilities
        for cap in &self.capabilities_drop {
            if !self.is_valid_capability(cap) {
                return Err(SecurityProfileError::InvalidCapability(format!("Invalid capability to drop: {}", cap)));
            }
        }

        for cap in &self.capabilities_add {
            if !self.is_valid_capability(cap) {
                return Err(SecurityProfileError::InvalidCapability(format!("Invalid capability to add: {}", cap)));
            }
        }

        // Validate dangerous capability combinations
        if self.capabilities_add.contains("ALL") && !self.capabilities_drop.is_empty() {
            return Err(SecurityProfileError::InvalidCapability(
                "Cannot drop capabilities when adding ALL".to_string()
            ));
        }

        // Validate security options
        for option in &self.security_options {
            if !self.is_valid_security_option(option) {
                return Err(SecurityProfileError::InvalidSecurityOption(format!("Invalid security option: {}", option)));
            }
        }

        // Validate paths
        for path in &self.read_only_paths {
            if !self.is_valid_path(path) {
                return Err(SecurityProfileError::InvalidPath(format!("Invalid read-only path: {}", path)));
            }
        }

        for path in &self.read_write_paths {
            if !self.is_valid_path(path) {
                return Err(SecurityProfileError::InvalidPath(format!("Invalid read-write path: {}", path)));
            }
        }

        // Validate security level requirements
        self.validate_security_level_requirements()?;

        Ok(())
    }

    /// Check if capability is valid
    fn is_valid_capability(&self, cap: &str) -> bool {
        // List of valid Linux capabilities
        let valid_caps = [
            "ALL", "AUDIT_CONTROL", "AUDIT_WRITE", "BLOCK_SUSPEND", "CHOWN", "DAC_OVERRIDE",
            "DAC_READ_SEARCH", "FOWNER", "FSETID", "IPC_LOCK", "IPC_OWNER", "KILL",
            "LEASE", "LINUX_IMMUTABLE", "MAC_ADMIN", "MAC_OVERRIDE", "MKNOD", "NET_ADMIN",
            "NET_BIND_SERVICE", "NET_BROADCAST", "NET_RAW", "SETGID", "SETFCAP", "SETPCAP",
            "SETUID", "SYS_ADMIN", "SYS_BOOT", "SYS_CHROOT", "SYS_MODULE", "SYS_NICE",
            "SYS_PACCT", "SYS_PTRACE", "SYS_RAWIO", "SYS_RESOURCE", "SYS_TIME", "SYS_TTY_CONFIG",
            "SYSLOG", "WAKE_ALARM"
        ];

        valid_caps.contains(&cap) || cap.chars().all(|c| c.is_uppercase() || c == '_')
    }

    /// Check if security option is valid
    fn is_valid_security_option(&self, option: &str) -> bool {
        let valid_options = [
            "no-new-privileges", "apparmor", "seccomp", "label", "userns", "mask", "readonly"
        ];

        valid_options.contains(&option) || option.starts_with("apparmor=") || option.starts_with("seccomp=")
    }

    /// Check if path is valid
    fn is_valid_path(&self, path: &str) -> bool {
        !path.is_empty() && path.starts_with('/')
    }

    /// Validate security level requirements
    fn validate_security_level_requirements(&self) -> Result<(), SecurityProfileError> {
        match self.security_level {
            SecurityLevel::Minimal => {
                // Minimal requirements
                if self.user == "root" || self.user == "0" {
                    return Err(SecurityProfileError::SecurityViolation(
                        "Minimal security level requires non-root user".to_string()
                    ));
                }
            }
            SecurityLevel::Standard => {
                // Standard requirements
                if !self.no_new_privileges {
                    return Err(SecurityProfileError::SecurityViolation(
                        "Standard security level requires no-new-privileges".to_string()
                    ));
                }
            }
            SecurityLevel::High => {
                // High requirements
                if !self.read_only {
                    return Err(SecurityProfileError::SecurityViolation(
                        "High security level requires read-only filesystem".to_string()
                    ));
                }
                if !self.no_new_privileges {
                    return Err(SecurityProfileError::SecurityViolation(
                        "High security level requires no-new-privileges".to_string()
                    ));
                }
                if self.capabilities_drop.is_empty() {
                    return Err(SecurityProfileError::SecurityViolation(
                        "High security level requires dropped capabilities".to_string()
                    ));
                }
            }
            SecurityLevel::Maximum => {
                // Maximum requirements
                if !self.read_only {
                    return Err(SecurityProfileError::SecurityViolation(
                        "Maximum security level requires read-only filesystem".to_string()
                    ));
                }
                if !self.no_new_privileges {
                    return Err(SecurityProfileError::SecurityViolation(
                        "Maximum security level requires no-new-privileges".to_string()
                    ));
                }
                if self.seccomp_profile.is_none() {
                    return Err(SecurityProfileError::SecurityViolation(
                        "Maximum security level requires seccomp profile".to_string()
                    ));
                }
            }
        }

        Ok(())
    }

    /// Generate Docker security arguments
    pub fn docker_args(&self) -> Result<Vec<String>, SecurityProfileError> {
        self.validate()?;

        let mut args = Vec::new();

        // User
        args.push("--user".to_string());
        args.push(self.user.clone());

        // Read-only filesystem
        if self.read_only {
            args.push("--read-only".to_string());
        }

        // Capabilities
        if !self.capabilities_drop.is_empty() {
            args.push("--cap-drop".to_string());
            args.push(self.capabilities_drop.iter().cloned().collect::<Vec<_>>().join(","));
        }

        if !self.capabilities_add.is_empty() {
            args.push("--cap-add".to_string());
            args.push(self.capabilities_add.iter().cloned().collect::<Vec<_>>().join(","));
        }

        // Security options
        for option in &self.security_options {
            args.push("--security-opt".to_string());
            args.push(option.clone());
        }

        // No new privileges
        if self.no_new_privileges && !self.security_options.contains(&"no-new-privileges".to_string()) {
            args.push("--security-opt".to_string());
            args.push("no-new-privileges".to_string());
        }

        // Namespaces
        if let Some(ref pid_ns) = self.pid_namespace {
            args.push("--pid".to_string());
            args.push(pid_ns.clone());
        }

        if let Some(ref network_ns) = self.network_namespace {
            args.push("--network".to_string());
            args.push(network_ns.clone());
        }

        if let Some(ref ipc_ns) = self.ipc_namespace {
            args.push("--ipc".to_string());
            args.push(ipc_ns.clone());
        }

        if let Some(ref uts_ns) = self.uts_namespace {
            args.push("--uts".to_string());
            args.push(uts_ns.clone());
        }

        if let Some(ref user_ns) = self.user_namespace {
            args.push("--userns".to_string());
            args.push(user_ns.clone());
        }

        // Seccomp profile
        if let Some(ref seccomp) = self.seccomp_profile {
            args.push("--security-opt".to_string());
            args.push(format!("seccomp={}", seccomp));
        }

        // AppArmor profile
        if let Some(ref apparmor) = self.apparmor_profile {
            args.push("--security-opt".to_string());
            args.push(format!("apparmor={}", apparmor));
        }

        // Read-only and read-write paths
        for path in &self.read_only_paths {
            args.push("--mount".to_string());
            args.push(format!("type=bind,source={},target={},readonly", path, path));
        }

        for path in &self.read_write_paths {
            args.push("--mount".to_string());
            args.push(format!("type=tmpfs,destination={}", path));
        }

        // Masked paths
        for path in &self.masked_paths {
            args.push("--security-opt".to_string());
            args.push(format!("mask={}", path));
        }

        Ok(args)
    }

    /// Calculate security score (0-100)
    pub fn security_score(&self) -> u8 {
        let mut score = 0;

        // User (30 points)
        if self.user != "root" && self.user != "0" && self.user != "0:0" {
            score += 30;
        }

        // Read-only filesystem (20 points)
        if self.read_only {
            score += 20;
        }

        // No new privileges (15 points)
        if self.no_new_privileges {
            score += 15;
        }

        // Capabilities (15 points)
        if self.capabilities_drop.contains("ALL") {
            score += 15;
        } else if !self.capabilities_drop.is_empty() {
            score += 10;
        }

        // Limited capabilities to add (5 points)
        if self.capabilities_add.len() <= 3 {
            score += 5;
        }

        // Security options (5 points)
        if !self.security_options.is_empty() {
            score += 5;
        }

        // Seccomp profile (5 points)
        if self.seccomp_profile.is_some() {
            score += 5;
        }

        // AppArmor profile (5 points)
        if self.apparmor_profile.is_some() {
            score += 5;
        }

        // Security level bonus
        match self.security_level {
            SecurityLevel::Maximum => score += 10,
            SecurityLevel::High => score += 5,
            _ => {}
        }

        score.min(100)
    }

    /// Check if profile meets compliance standards
    pub fn meets_compliance(&self, standard: &str) -> bool {
        self.compliance_standards.contains(standard)
    }

    /// Get compliance status for all standards
    pub fn compliance_status(&self) -> std::collections::HashMap<String, bool> {
        let mut status = std::collections::HashMap::new();

        for standard in &self.compliance_standards {
            status.insert(standard.clone(), true);
        }

        // Check specific compliance requirements
        if self.meets_compliance("CIS_Docker_Benchmark") {
            status.insert("CIS_Docker_Benchmark".to_string(), self.cis_compliant());
        }

        if self.meets_compliance("NIST_800_190") {
            status.insert("NIST_800_190".to_string(), self.nist_compliant());
        }

        status
    }

    /// Check CIS Docker Benchmark compliance
    fn cis_compliant(&self) -> bool {
        self.user != "root" &&
        self.no_new_privileges &&
        !self.capabilities_add.contains("ALL") &&
        self.security_score() >= 70
    }

    /// Check NIST 800-190 compliance
    fn nist_compliant(&self) -> bool {
        self.user != "root" &&
        self.no_new_privileges &&
        self.read_only &&
        self.seccomp_profile.is_some() &&
        self.security_score() >= 80
    }
}

impl fmt::Display for SecurityProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SecurityProfile(user={}, read_only={}, no_new_privileges={}, level={}, score={})",
            self.user,
            self.read_only,
            self.no_new_privileges,
            self.security_level,
            self.security_score()
        )
    }
}

/// Security levels for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Minimal security (basic protections)
    Minimal,
    /// Standard security (recommended for most use cases)
    Standard,
    /// High security (production workloads)
    High,
    /// Maximum security (highly sensitive data)
    Maximum,
}

impl fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityLevel::Minimal => write!(f, "minimal"),
            SecurityLevel::Standard => write!(f, "standard"),
            SecurityLevel::High => write!(f, "high"),
            SecurityLevel::Maximum => write!(f, "maximum"),
        }
    }
}

/// Errors related to security profiles
#[derive(Debug, thiserror::Error)]
pub enum SecurityProfileError {
    #[error("Invalid user: {0}")]
    InvalidUser(String),

    #[error("Invalid capability: {0}")]
    InvalidCapability(String),

    #[error("Invalid security option: {0}")]
    InvalidSecurityOption(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_security_profile() {
        let profile = SecurityProfile::default();
        assert_eq!(profile.user, "1000:1000");
        assert!(profile.read_only);
        assert!(profile.no_new_privileges);
        assert!(profile.capabilities_drop.contains("ALL"));
        assert_eq!(profile.security_level, SecurityLevel::High);
    }

    #[test]
    fn test_minimal_security_profile() {
        let profile = SecurityProfile::minimal();
        assert_eq!(profile.user, "1000:1000");
        assert!(!profile.read_only);
        assert!(!profile.no_new_privileges);
        assert!(profile.capabilities_drop.is_empty());
        assert_eq!(profile.security_level, SecurityLevel::Minimal);
    }

    #[test]
    fn test_standard_security_profile() {
        let profile = SecurityProfile::standard();
        assert_eq!(profile.user, "1000:1000");
        assert!(!profile.read_only);
        assert!(profile.no_new_privileges);
        assert!(!profile.capabilities_drop.is_empty());
        assert_eq!(profile.security_level, SecurityLevel::Standard);
    }

    #[test]
    fn test_maximum_security_profile() {
        let profile = SecurityProfile::maximum();
        assert!(profile.read_only);
        assert!(profile.no_new_privileges);
        assert!(profile.seccomp_profile.is_some());
        assert!(profile.apparmor_profile.is_some());
        assert_eq!(profile.security_level, SecurityLevel::Maximum);
    }

    #[test]
    fn test_with_user() {
        let profile = SecurityProfile::minimal().with_user("1001:1001".to_string());
        assert_eq!(profile.user, "1001:1001");
    }

    #[test]
    fn test_read_only() {
        let profile = SecurityProfile::minimal().read_only();
        assert!(profile.read_only);
    }

    #[test]
    fn test_drop_capability() {
        let profile = SecurityProfile::minimal().drop_capability("NET_RAW".to_string());
        assert!(profile.capabilities_drop.contains("NET_RAW"));
    }

    #[test]
    fn test_add_capability() {
        let profile = SecurityProfile::minimal().add_capability("CHOWN".to_string());
        assert!(profile.capabilities_add.contains("CHOWN"));
    }

    #[test]
    fn test_validate_valid_profile() {
        let profile = SecurityProfile::standard();
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn test_validate_root_user() {
        let profile = SecurityProfile::minimal().with_user("root".to_string());
        assert!(matches!(profile.validate(), Err(SecurityProfileError::SecurityViolation(_))));
    }

    #[test]
    fn test_validate_invalid_capability() {
        let profile = SecurityProfile::minimal().drop_capability("INVALID_CAP".to_string());
        assert!(matches!(profile.validate(), Err(SecurityProfileError::InvalidCapability(_))));
    }

    #[test]
    fn test_security_score() {
        let minimal = SecurityProfile::minimal();
        let maximum = SecurityProfile::maximum();

        assert!(maximum.security_score() > minimal.security_score());
    }

    #[test]
    fn test_cis_compliance() {
        let compliant = SecurityProfile::standard();
        let non_compliant = SecurityProfile::minimal();

        assert!(compliant.cis_compliant());
        assert!(!non_compliant.cis_compliant());
    }

    #[test]
    fn test_nist_compliance() {
        let compliant = SecurityProfile::high();
        let non_compliant = SecurityProfile::standard();

        assert!(compliant.nist_compliant());
        assert!(!non_compliant.nist_compliant());
    }

    #[test]
    fn test_docker_args() {
        let profile = SecurityProfile::standard();
        let args = profile.docker_args().unwrap();

        assert!(args.contains(&"--user".to_string()));
        assert!(args.contains(&"--security-opt".to_string()));
        assert!(args.contains(&"no-new-privileges".to_string()));
    }

    #[test]
    fn test_display() {
        let profile = SecurityProfile::standard();
        let display = format!("{}", profile);
        assert!(display.contains("user=1000:1000"));
        assert!(display.contains("read_only=false"));
        assert!(display.contains("level=standard"));
    }
}