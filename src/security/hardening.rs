//! Security hardening module for Merlin AI Router Docker deployment
//! Provides comprehensive security controls and hardening measures

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use crate::models::container_config::DockerContainerConfig;
use crate::models::security_scan_config::{SecurityScanConfig, ComplianceStandard};
use super::docker_client::DockerConfigError;

/// Security hardening configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHardeningConfig {
    pub enable_seccomp: bool,
    pub enable_apparmor: bool,
    pub enable_selinux: bool,
    pub read_only_root_filesystem: bool,
    pub drop_all_capabilities: bool,
    pub no_new_privileges: bool,
    pub user_namespace_remap: bool,
    pub network_mode: NetworkMode,
    pub resource_limits: ResourceLimits,
    pub security_context: SecurityContext,
    pub compliance_standards: Vec<ComplianceStandard>,
}

/// Network security modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetworkMode {
    None,
    Host,
    Bridge,
    Custom(String),
}

/// Resource limits for security hardening
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_cores: f64,
    pub max_pids: u64,
    pub max_open_files: u64,
    pub max_processes: u64,
}

/// Security context configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    pub run_as_user: Option<u32>,
    pub run_as_group: Option<u32>,
    pub fs_group: Option<u32>,
    pub privileged: bool,
    pub allow_privilege_escalation: bool,
    pub read_only_root_filesystem: bool,
    pub run_as_non_root: bool,
    pub capabilities: CapabilityConfig,
}

/// Capability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityConfig {
    pub add: Vec<String>,
    pub drop: Vec<String>,
}

/// Security hardening result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardeningResult {
    pub id: Uuid,
    pub config_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub applied_controls: Vec<SecurityControl>,
    pub failed_controls: Vec<SecurityControl>,
    pub compliance_score: f64,
    pub vulnerabilities_found: u32,
    pub recommendations: Vec<String>,
}

/// Security control types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SecurityControl {
    SeccompProfile,
    AppArmorProfile,
    SELinuxContext,
    ReadOnlyFilesystem,
    CapabilityDropping,
    NonRootUser,
    NetworkIsolation,
    ResourceLimiting,
    NamespaceIsolation,
    MountHardening,
    Custom(String),
}

/// Security hardening service
#[derive(Clone)]
pub struct SecurityHardeningService {
    hardening_configs: Arc<RwLock<HashMap<Uuid, SecurityHardeningConfig>>>,
    hardening_results: Arc<RwLock<HashMap<Uuid, HardeningResult>>>,
    security_profiles: Arc<RwLock<HashMap<String, SecurityProfile>>>,
    compliance_framework: ComplianceFramework,
}

/// Security profile for container hardening
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProfile {
    pub name: String,
    pub description: String,
    pub controls: Vec<SecurityControl>,
    pub configuration: SecurityHardeningConfig,
    pub compliance_standards: Vec<ComplianceStandard>,
}

/// Compliance framework
#[derive(Debug, Clone)]
pub struct ComplianceFramework {
    standards: HashMap<ComplianceStandard, ComplianceRequirement>,
}

/// Compliance requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequirement {
    pub standard: ComplianceStandard,
    pub controls: Vec<SecurityControl>,
    pub scoring_rules: ScoringRules,
}

/// Scoring rules for compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringRules {
    pub control_weights: HashMap<SecurityControl, f64>,
    pub passing_threshold: f64,
    pub critical_controls: Vec<SecurityControl>,
}

impl SecurityHardeningService {
    pub fn new() -> Self {
        Self {
            hardening_configs: Arc::new(RwLock::new(HashMap::new())),
            hardening_results: Arc::new(RwLock::new(HashMap::new())),
            security_profiles: Arc::new(RwLock::new(HashMap::new())),
            compliance_framework: ComplianceFramework::new(),
        }
    }

    /// Apply security hardening to a container configuration
    pub async fn apply_hardening(
        &self,
        config: &DockerContainerConfig,
        hardening_config: SecurityHardeningConfig,
    ) -> Result<HardeningResult, DockerConfigError> {
        let mut applied_controls = Vec::new();
        let mut failed_controls = Vec::new();
        let mut recommendations = Vec::new();
        let mut vulnerabilities_found = 0;

        // Apply each security control
        if hardening_config.enable_seccomp {
            if self.apply_seccomp_profile(config).await {
                applied_controls.push(SecurityControl::SeccompProfile);
            } else {
                failed_controls.push(SecurityControl::SeccompProfile);
                recommendations.push("Enable seccomp profile for system call filtering".to_string());
            }
        }

        if hardening_config.enable_apparmor {
            if self.apply_apparmor_profile(config).await {
                applied_controls.push(SecurityControl::AppArmorProfile);
            } else {
                failed_controls.push(SecurityControl::AppArmorProfile);
                recommendations.push("Enable AppArmor profile for mandatory access control".to_string());
            }
        }

        if hardening_config.read_only_root_filesystem {
            if self.apply_readonly_filesystem(config).await {
                applied_controls.push(SecurityControl::ReadOnlyFilesystem);
            } else {
                failed_controls.push(SecurityControl::ReadOnlyFilesystem);
                recommendations.push("Use read-only root filesystem for better security".to_string());
            }
        }

        if hardening_config.security_context.run_as_non_root {
            if self.apply_non_root_user(config).await {
                applied_controls.push(SecurityControl::NonRootUser);
            } else {
                failed_controls.push(SecurityControl::NonRootUser);
                recommendations.push("Run containers as non-root user".to_string());
                vulnerabilities_found += 1;
            }
        }

        if hardening_config.drop_all_capabilities {
            if self.apply_capability_dropping(config).await {
                applied_controls.push(SecurityControl::CapabilityDropping);
            } else {
                failed_controls.push(SecurityControl::CapabilityDropping);
                recommendations.push("Drop unnecessary capabilities".to_string());
            }
        }

        // Apply resource limits
        if self.apply_resource_limits(config, &hardening_config.resource_limits).await {
            applied_controls.push(SecurityControl::ResourceLimiting);
        } else {
            failed_controls.push(SecurityControl::ResourceLimiting);
            recommendations.push("Apply resource limits to prevent resource exhaustion".to_string());
        }

        // Calculate compliance score
        let compliance_score = self.calculate_compliance_score(
            &applied_controls,
            &hardening_config.compliance_standards,
        );

        let result = HardeningResult {
            id: Uuid::new_v4(),
            config_id: config.id,
            timestamp: chrono::Utc::now(),
            applied_controls,
            failed_controls,
            compliance_score,
            vulnerabilities_found,
            recommendations,
        };

        // Store result
        let mut results = self.hardening_results.write().await;
        results.insert(result.id, result.clone());

        Ok(result)
    }

    /// Apply seccomp profile
    async fn apply_seccomp_profile(&self, config: &DockerContainerConfig) -> bool {
        // In a real implementation, this would configure seccomp
        // For now, we'll simulate the application
        config.security_options.iter().any(|opt| opt.contains("seccomp"))
    }

    /// Apply AppArmor profile
    async fn apply_apparmor_profile(&self, config: &DockerContainerConfig) -> bool {
        // In a real implementation, this would configure AppArmor
        config.security_options.iter().any(|opt| opt.contains("apparmor"))
    }

    /// Apply read-only filesystem
    async fn apply_readonly_filesystem(&self, config: &DockerContainerConfig) -> bool {
        // Check if read-only filesystem is configured
        config.read_only.unwrap_or(false)
    }

    /// Apply non-root user
    async fn apply_non_root_user(&self, config: &DockerContainerConfig) -> bool {
        // Check if container is configured to run as non-root
        config.user.as_deref().unwrap_or("root") != "root"
    }

    /// Apply capability dropping
    async fn apply_capability_dropping(&self, config: &DockerContainerConfig) -> bool {
        // Check if capabilities are properly restricted
        config.cap_drop.as_ref().map_or(false, |caps| !caps.is_empty())
    }

    /// Apply resource limits
    async fn apply_resource_limits(&self, config: &DockerContainerConfig, limits: &ResourceLimits) -> bool {
        // Check if resource limits are configured
        let has_memory_limit = config.host_config.memory.unwrap_or(0) > 0;
        let has_cpu_limit = config.host_config.cpu_shares.unwrap_or(0) > 0;
        let has_pids_limit = config.host_config.pids_limit.unwrap_or(0) > 0;

        has_memory_limit && has_cpu_limit && has_pids_limit
    }

    /// Calculate compliance score
    fn calculate_compliance_score(
        &self,
        applied_controls: &[SecurityControl],
        standards: &[ComplianceStandard],
    ) -> f64 {
        let mut total_score = 0.0;
        let mut max_score = 0.0;

        for standard in standards {
            if let Some(requirements) = self.compliance_framework.standards.get(standard) {
                let standard_score = self.calculate_standard_score(applied_controls, requirements);
                total_score += standard_score;
                max_score += 100.0; // Each standard is scored out of 100
            }
        }

        if max_score > 0.0 {
            (total_score / max_score) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate score for a specific standard
    fn calculate_standard_score(
        &self,
        applied_controls: &[SecurityControl],
        requirements: &ComplianceRequirement,
    ) -> f64 {
        let mut score = 0.0;
        let mut max_score = 0.0;

        for (control, weight) in &requirements.scoring_rules.control_weights {
            max_score += weight;
            if applied_controls.contains(control) {
                score += weight;
            }
        }

        if max_score > 0.0 {
            (score / max_score) * 100.0
        } else {
            0.0
        }
    }

    /// Get security profile by name
    pub async fn get_security_profile(&self, name: &str) -> Option<SecurityProfile> {
        let profiles = self.security_profiles.read().await;
        profiles.get(name).cloned()
    }

    /// Create custom security profile
    pub async fn create_security_profile(&self, profile: SecurityProfile) -> Result<(), DockerConfigError> {
        let mut profiles = self.security_profiles.write().await;
        profiles.insert(profile.name.clone(), profile);
        Ok(())
    }

    /// Get all available security profiles
    pub async fn get_security_profiles(&self) -> Vec<SecurityProfile> {
        let profiles = self.security_profiles.read().await;
        profiles.values().cloned().collect()
    }

    /// Get hardening result by ID
    pub async fn get_hardening_result(&self, result_id: Uuid) -> Option<HardeningResult> {
        let results = self.hardening_results.read().await;
        results.get(&result_id).cloned()
    }

    /// Get hardening results for a configuration
    pub async fn get_config_hardening_results(&self, config_id: Uuid) -> Vec<HardeningResult> {
        let results = self.hardening_results.read().await;
        results
            .values()
            .filter(|r| r.config_id == config_id)
            .cloned()
            .collect()
    }

    /// Generate security hardening recommendations
    pub async fn generate_recommendations(&self, config: &DockerContainerConfig) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Check for common security issues
        if !config.read_only.unwrap_or(false) {
            recommendations.push("Enable read-only root filesystem".to_string());
        }

        if config.user.as_deref().unwrap_or("root") == "root" {
            recommendations.push("Run container as non-root user".to_string());
        }

        if config.privileged.unwrap_or(false) {
            recommendations.push("Disable privileged mode".to_string());
        }

        if config.cap_drop.as_ref().map_or(true, |caps| caps.is_empty()) {
            recommendations.push("Drop unnecessary capabilities".to_string());
        }

        if config.host_config.memory.unwrap_or(0) == 0 {
            recommendations.push("Set memory limits".to_string());
        }

        if config.host_config.cpu_shares.unwrap_or(0) == 0 {
            recommendations.push("Set CPU limits".to_string());
        }

        recommendations
    }

    /// Initialize default security profiles
    pub async fn initialize_default_profiles(&self) -> Result<(), DockerConfigError> {
        let profiles = vec![
            SecurityProfile {
                name: "minimal".to_string(),
                description: "Minimal security profile for basic hardening".to_string(),
                controls: vec![
                    SecurityControl::NonRootUser,
                    SecurityControl::ReadOnlyFilesystem,
                    SecurityControl::CapabilityDropping,
                ],
                configuration: SecurityHardeningConfig {
                    enable_seccomp: true,
                    enable_apparmor: true,
                    enable_selinux: false,
                    read_only_root_filesystem: true,
                    drop_all_capabilities: true,
                    no_new_privileges: true,
                    user_namespace_remap: false,
                    network_mode: NetworkMode::Bridge,
                    resource_limits: ResourceLimits {
                        max_memory_mb: 512,
                        max_cpu_cores: 1.0,
                        max_pids: 100,
                        max_open_files: 1024,
                        max_processes: 50,
                    },
                    security_context: SecurityContext {
                        run_as_user: Some(1000),
                        run_as_group: Some(1000),
                        fs_group: Some(1000),
                        privileged: false,
                        allow_privilege_escalation: false,
                        read_only_root_filesystem: true,
                        run_as_non_root: true,
                        capabilities: CapabilityConfig {
                            add: vec![],
                            drop: vec!["ALL".to_string()],
                        },
                    },
                    compliance_standards: vec![ComplianceStandard::CISDockerBenchmark],
                },
            },
            SecurityProfile {
                name: "strict".to_string(),
                description: "Strict security profile for maximum hardening".to_string(),
                controls: vec![
                    SecurityControl::SeccompProfile,
                    SecurityControl::AppArmorProfile,
                    SecurityControl::SELinuxContext,
                    SecurityControl::ReadOnlyFilesystem,
                    SecurityControl::CapabilityDropping,
                    SecurityControl::NonRootUser,
                    SecurityControl::NetworkIsolation,
                    SecurityControl::ResourceLimiting,
                    SecurityControl::NamespaceIsolation,
                ],
                configuration: SecurityHardeningConfig {
                    enable_seccomp: true,
                    enable_apparmor: true,
                    enable_selinux: true,
                    read_only_root_filesystem: true,
                    drop_all_capabilities: true,
                    no_new_privileges: true,
                    user_namespace_remap: true,
                    network_mode: NetworkMode::None,
                    resource_limits: ResourceLimits {
                        max_memory_mb: 256,
                        max_cpu_cores: 0.5,
                        max_pids: 50,
                        max_open_files: 512,
                        max_processes: 25,
                    },
                    security_context: SecurityContext {
                        run_as_user: Some(1000),
                        run_as_group: Some(1000),
                        fs_group: Some(1000),
                        privileged: false,
                        allow_privilege_escalation: false,
                        read_only_root_filesystem: true,
                        run_as_non_root: true,
                        capabilities: CapabilityConfig {
                            add: vec![],
                            drop: vec!["ALL".to_string()],
                        },
                    },
                    compliance_standards: vec![
                        ComplianceStandard::CISDockerBenchmark,
                        ComplianceStandard::NIST800190,
                        ComplianceStandard::PCIDSS,
                    ],
                },
            },
        ];

        let mut security_profiles = self.security_profiles.write().await;
        for profile in profiles {
            security_profiles.insert(profile.name.clone(), profile);
        }

        Ok(())
    }
}

impl ComplianceFramework {
    pub fn new() -> Self {
        let mut standards = HashMap::new();

        // CIS Docker Benchmark
        standards.insert(
            ComplianceStandard::CISDockerBenchmark,
            ComplianceRequirement {
                standard: ComplianceStandard::CISDockerBenchmark,
                controls: vec![
                    SecurityControl::NonRootUser,
                    SecurityControl::ReadOnlyFilesystem,
                    SecurityControl::CapabilityDropping,
                    SecurityControl::ResourceLimiting,
                ],
                scoring_rules: ScoringRules {
                    control_weights: {
                        let mut weights = HashMap::new();
                        weights.insert(SecurityControl::NonRootUser, 25.0);
                        weights.insert(SecurityControl::ReadOnlyFilesystem, 25.0);
                        weights.insert(SecurityControl::CapabilityDropping, 25.0);
                        weights.insert(SecurityControl::ResourceLimiting, 25.0);
                        weights
                    },
                    passing_threshold: 80.0,
                    critical_controls: vec![
                        SecurityControl::NonRootUser,
                        SecurityControl::ReadOnlyFilesystem,
                    ],
                },
            },
        );

        // NIST 800-190
        standards.insert(
            ComplianceStandard::NIST800190,
            ComplianceRequirement {
                standard: ComplianceStandard::NIST800190,
                controls: vec![
                    SecurityControl::SeccompProfile,
                    SecurityControl::AppArmorProfile,
                    SecurityControl::NamespaceIsolation,
                    SecurityControl::MountHardening,
                ],
                scoring_rules: ScoringRules {
                    control_weights: {
                        let mut weights = HashMap::new();
                        weights.insert(SecurityControl::SeccompProfile, 25.0);
                        weights.insert(SecurityControl::AppArmorProfile, 25.0);
                        weights.insert(SecurityControl::NamespaceIsolation, 25.0);
                        weights.insert(SecurityControl::MountHardening, 25.0);
                        weights
                    },
                    passing_threshold: 75.0,
                    critical_controls: vec![
                        SecurityControl::SeccompProfile,
                        SecurityControl::AppArmorProfile,
                    ],
                },
            },
        );

        Self { standards }
    }
}

impl Default for SecurityHardeningConfig {
    fn default() -> Self {
        Self {
            enable_seccomp: true,
            enable_apparmor: true,
            enable_selinux: false,
            read_only_root_filesystem: true,
            drop_all_capabilities: true,
            no_new_privileges: true,
            user_namespace_remap: false,
            network_mode: NetworkMode::Bridge,
            resource_limits: ResourceLimits {
                max_memory_mb: 512,
                max_cpu_cores: 1.0,
                max_pids: 100,
                max_open_files: 1024,
                max_processes: 50,
            },
            security_context: SecurityContext {
                run_as_user: Some(1000),
                run_as_group: Some(1000),
                fs_group: Some(1000),
                privileged: false,
                allow_privilege_escalation: false,
                read_only_root_filesystem: true,
                run_as_non_root: true,
                capabilities: CapabilityConfig {
                    add: vec![],
                    drop: vec!["ALL".to_string()],
                },
            },
            compliance_standards: vec![ComplianceStandard::CISDockerBenchmark],
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_cores: 1.0,
            max_pids: 100,
            max_open_files: 1024,
            max_processes: 50,
        }
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            run_as_user: Some(1000),
            run_as_group: Some(1000),
            fs_group: Some(1000),
            privileged: false,
            allow_privilege_escalation: false,
            read_only_root_filesystem: true,
            run_as_non_root: true,
            capabilities: CapabilityConfig {
                add: vec![],
                drop: vec!["ALL".to_string()],
            },
        }
    }
}

impl Default for CapabilityConfig {
    fn default() -> Self {
        Self {
            add: vec![],
            drop: vec!["ALL".to_string()],
        }
    }
}