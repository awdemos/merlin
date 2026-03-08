use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Deployment environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentEnvironment {
    /// Unique identifier for this environment
    pub id: Uuid,

    /// Environment name
    pub name: String,

    /// Environment type (development, staging, production)
    pub environment_type: EnvironmentType,

    /// Description of the environment
    pub description: String,

    /// Environment-specific configuration
    pub configuration: EnvironmentConfiguration,

    /// Resource limits for this environment
    pub resource_limits: Option<ResourceLimits>,

    /// Security profile for this environment
    pub security_profile: Option<SecurityProfile>,

    /// Network configuration for this environment
    pub network_config: Option<NetworkConfig>,

    /// Health check configuration for this environment
    pub health_check: Option<HealthCheckConfig>,

    /// Environment variables specific to this environment
    pub environment_variables: HashMap<String, String>,

    /// Deployment strategy
    pub deployment_strategy: DeploymentStrategy,

    /// Auto-scaling configuration
    pub auto_scaling: Option<AutoScalingConfig>,

    /// Monitoring and logging configuration
    pub monitoring: MonitoringConfig,

    /// Backup configuration
    pub backup_config: Option<BackupConfig>,

    /// Compliance requirements for this environment
    pub compliance_requirements: Vec<ComplianceRequirement>,

    /// Tags for this environment
    pub tags: Vec<String>,

    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Last deployed timestamp
    pub last_deployed_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Current deployment status
    pub deployment_status: DeploymentStatus,
}

impl Default for DeploymentEnvironment {
    fn default() -> Self {
        let now = chrono::Utc::now();

        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            environment_type: EnvironmentType::Development,
            description: String::new(),
            configuration: EnvironmentConfiguration::default(),
            resource_limits: None,
            security_profile: None,
            network_config: None,
            health_check: None,
            environment_variables: HashMap::new(),
            deployment_strategy: DeploymentStrategy::RollingUpdate,
            auto_scaling: None,
            monitoring: MonitoringConfig::default(),
            backup_config: None,
            compliance_requirements: Vec::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            last_deployed_at: None,
            deployment_status: DeploymentStatus::NotDeployed,
        }
    }
}

impl DeploymentEnvironment {
    /// Create a new deployment environment
    pub fn new(name: String, environment_type: EnvironmentType) -> Self {
        let mut env = Self::default();
        env.name = name;
        env.environment_type = environment_type;
        env
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Set environment configuration
    pub fn with_configuration(mut self, config: EnvironmentConfiguration) -> Self {
        self.configuration = config;
        self
    }

    /// Set resource limits
    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = Some(limits);
        self
    }

    /// Set security profile
    pub fn with_security_profile(mut self, profile: SecurityProfile) -> Self {
        self.security_profile = Some(profile);
        self
    }

    /// Set network configuration
    pub fn with_network_config(mut self, config: NetworkConfig) -> Self {
        self.network_config = Some(config);
        self
    }

    /// Set health check configuration
    pub fn with_health_check(mut self, health_check: HealthCheckConfig) -> Self {
        self.health_check = Some(health_check);
        self
    }

    /// Add environment variable
    pub fn add_environment_variable(mut self, key: String, value: String) -> Self {
        self.environment_variables.insert(key, value);
        self
    }

    /// Set deployment strategy
    pub fn with_deployment_strategy(mut self, strategy: DeploymentStrategy) -> Self {
        self.deployment_strategy = strategy;
        self
    }

    /// Set auto-scaling configuration
    pub fn with_auto_scaling(mut self, scaling: AutoScalingConfig) -> Self {
        self.auto_scaling = Some(scaling);
        self
    }

    /// Set monitoring configuration
    pub fn with_monitoring(mut self, monitoring: MonitoringConfig) -> Self {
        self.monitoring = monitoring;
        self
    }

    /// Set backup configuration
    pub fn with_backup_config(mut self, backup: BackupConfig) -> Self {
        self.backup_config = Some(backup);
        self
    }

    /// Add compliance requirement
    pub fn add_compliance_requirement(mut self, requirement: ComplianceRequirement) -> Self {
        self.compliance_requirements.push(requirement);
        self
    }

    /// Add tag
    pub fn add_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Validate the environment configuration
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        if self.name.is_empty() {
            return Err(DeploymentEnvironmentError::InvalidName("Environment name cannot be empty".to_string()));
        }

        // Validate environment configuration
        self.configuration.validate()?;

        // Validate resource limits if present
        if let Some(ref limits) = self.resource_limits {
            limits.validate()?;
        }

        // Validate security profile if present
        if let Some(ref profile) = self.security_profile {
            profile.validate()?;
        }

        // Validate network configuration if present
        if let Some(ref network) = self.network_config {
            network.validate()?;
        }

        // Validate health check if present
        if let Some(ref health) = self.health_check {
            health.validate()?;
        }

        // Validate auto-scaling if present
        if let Some(ref scaling) = self.auto_scaling {
            scaling.validate()?;
        }

        // Validate monitoring configuration
        self.monitoring.validate()?;

        // Validate backup configuration if present
        if let Some(ref backup) = self.backup_config {
            backup.validate()?;
        }

        // Validate environment-specific requirements
        self.validate_environment_specific_requirements()?;

        Ok(())
    }

    /// Validate environment-specific requirements
    fn validate_environment_specific_requirements(&self) -> Result<(), DeploymentEnvironmentError> {
        match self.environment_type {
            EnvironmentType::Development => {
                // Development environments should have minimal resource limits
                if let Some(ref limits) = self.resource_limits {
                    if limits.memory_mb > 2048 {
                        return Err(DeploymentEnvironmentError::InvalidConfiguration(
                            "Development environment should not exceed 2GB memory".to_string()
                        ));
                    }
                }
            }
            EnvironmentType::Staging => {
                // Staging environments should mirror production with reduced scale
                if let Some(ref limits) = self.resource_limits {
                    if limits.memory_mb > 4096 {
                        return Err(DeploymentEnvironmentError::InvalidConfiguration(
                            "Staging environment should not exceed 4GB memory".to_string()
                        ));
                    }
                }
            }
            EnvironmentType::Production => {
                // Production environments should have resource limits
                if self.resource_limits.is_none() {
                    return Err(DeploymentEnvironmentError::InvalidConfiguration(
                        "Production environment must have resource limits".to_string()
                    ));
                }

                // Production environments should have security profiles
                if self.security_profile.is_none() {
                    return Err(DeploymentEnvironmentError::InvalidConfiguration(
                        "Production environment must have security profile".to_string()
                    ));
                }

                // Production environments should have health checks
                if self.health_check.is_none() {
                    return Err(DeploymentEnvironmentError::InvalidConfiguration(
                        "Production environment must have health checks".to_string()
                    ));
                }

                // Production environments should have backups
                if self.backup_config.is_none() {
                    return Err(DeploymentEnvironmentError::InvalidConfiguration(
                        "Production environment must have backup configuration".to_string()
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if this environment is ready for deployment
    pub fn is_ready_for_deployment(&self) -> bool {
        self.validate().is_ok() &&
        self.deployment_status != DeploymentStatus::Deploying &&
        self.deployment_status != DeploymentStatus::Failed
    }

    /// Check if this environment is currently deployed
    pub fn is_deployed(&self) -> bool {
        matches!(self.deployment_status, DeploymentStatus::Deployed | DeploymentStatus::Deploying)
    }

    /// Generate environment-specific Docker run arguments
    pub fn docker_run_args(&self) -> Result<Vec<String>, DeploymentEnvironmentError> {
        self.validate()?;

        let mut args = Vec::new();

        // Add environment variables
        for (key, value) in &self.environment_variables {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Add resource limits
        if let Some(ref limits) = self.resource_limits {
            args.push("--memory".to_string());
            args.push(format!("{}m", limits.memory_mb));

            args.push("--cpus".to_string());
            args.push(format!("{}", limits.cpu_shares));

            if limits.pids_limit > 0 {
                args.push("--pids-limit".to_string());
                args.push(format!("{}", limits.pids_limit));
            }
        }

        // Add security arguments
        if let Some(ref profile) = self.security_profile {
            args.extend(profile.docker_args()?);
        }

        // Add network configuration
        if let Some(ref network) = self.network_config {
            args.extend(network.docker_args()?);
        }

        // Add health check
        if let Some(ref health) = self.health_check {
            args.extend(health.docker_args()?);
        }

        Ok(args)
    }

    /// Generate environment-specific Docker build arguments
    pub fn docker_build_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Add build arguments based on environment type
        args.push("--build-arg".to_string());
        args.push(format!("RUST_ENV={}", self.environment_type.to_string().to_lowercase()));

        args.push("--build-arg".to_string());
        args.push(format!("MERLIN_ENV={}", self.environment_type.to_string().to_lowercase()));

        // Add custom build arguments
        for (key, value) in &self.configuration.build_args {
            args.push("--build-arg".to_string());
            args.push(format!("{}={}", key, value));
        }

        args
    }

    /// Calculate environment health score (0-100)
    pub fn health_score(&self) -> u8 {
        let mut score = 100;

        // Deployment status affects score
        match self.deployment_status {
            DeploymentStatus::Deployed => score += 10,
            DeploymentStatus::Deploying => score -= 5,
            DeploymentStatus::Failed => score -= 30,
            DeploymentStatus::NotDeployed => score -= 20,
        }

        // Resource usage affects score
        if let Some(ref limits) = self.resource_limits {
            if limits.memory_mb > 4096 {
                score -= 10;
            }
            if limits.cpu_shares > 4.0 {
                score -= 10;
            }
        }

        // Security profile affects score
        if let Some(ref profile) = self.security_profile {
            let security_score = profile.security_score();
            score = (score + security_score) / 2;
        }

        // Monitoring affects score
        if self.monitoring.enabled {
            score += 5;
        }

        // Backup configuration affects score
        if self.backup_config.is_some() {
            score += 10;
        }

        // Time since last deployment affects score
        if let Some(last_deployed) = self.last_deployed_at {
            let days_since_deploy = (chrono::Utc::now() - last_deployed).num_days();
            if days_since_deploy > 30 {
                score -= 10;
            }
        }

        score.max(0).min(100)
    }

    /// Check if environment meets compliance requirements
    pub fn meets_compliance_requirements(&self) -> bool {
        self.compliance_requirements.is_empty() ||
        self.compliance_requirements.iter().all(|req| req.met)
    }

    /// Clone and adapt for a different environment type
    pub fn adapt_for_environment(&self, new_type: EnvironmentType) -> Self {
        let mut adapted = self.clone();
        adapted.environment_type = new_type;
        adapted.id = Uuid::new_v4();
        adapted.created_at = chrono::Utc::now();
        adapted.updated_at = chrono::Utc::now();
        adapted.last_deployed_at = None;
        adapted.deployment_status = DeploymentStatus::NotDeployed;

        // Adapt resource limits based on environment type
        match new_type {
            EnvironmentType::Development => {
                adapted.resource_limits = Some(ResourceLimits::minimal());
                adapted.security_profile = Some(SecurityProfile::minimal());
            }
            EnvironmentType::Staging => {
                adapted.resource_limits = Some(ResourceLimits::standard());
                adapted.security_profile = Some(SecurityProfile::standard());
            }
            EnvironmentType::Production => {
                adapted.resource_limits = Some(ResourceLimits::high_performance());
                adapted.security_profile = Some(SecurityProfile::high());
            }
        }

        adapted
    }
}

impl fmt::Display for DeploymentEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DeploymentEnvironment(name={}, type={}, status={}, health_score={})",
            self.name,
            self.environment_type,
            self.deployment_status,
            self.health_score()
        )
    }
}

/// Environment types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvironmentType {
    Development,
    Staging,
    Production,
}

impl fmt::Display for EnvironmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvironmentType::Development => write!(f, "Development"),
            EnvironmentType::Staging => write!(f, "Staging"),
            EnvironmentType::Production => write!(f, "Production"),
        }
    }
}

/// Environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfiguration {
    /// Build arguments for Docker builds
    pub build_args: HashMap<String, String>,

    /// Docker registry configuration
    pub registry: RegistryConfig,

    /// Image tagging strategy
    pub tagging_strategy: TaggingStrategy,

    /// Cleanup policy for old images
    pub cleanup_policy: CleanupPolicy,

    /// Environment-specific labels
    pub labels: HashMap<String, String>,
}

impl Default for EnvironmentConfiguration {
    fn default() -> Self {
        let mut build_args = HashMap::new();
        build_args.insert("RUST_ENV".to_string(), "development".to_string());
        build_args.insert("MERLIN_ENV".to_string(), "development".to_string());

        Self {
            build_args,
            registry: RegistryConfig::default(),
            tagging_strategy: TaggingStrategy::CommitHash,
            cleanup_policy: CleanupPolicy::default(),
            labels: HashMap::new(),
        }
    }
}

impl EnvironmentConfiguration {
    /// Validate environment configuration
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        // Validate registry configuration
        self.registry.validate()?;

        // Validate cleanup policy
        self.cleanup_policy.validate()?;

        Ok(())
    }
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry URL
    pub url: String,

    /// Registry username (optional)
    pub username: Option<String>,

    /// Registry password (optional)
    pub password: Option<String>,

    /// Whether to use TLS
    pub use_tls: bool,

    /// Image namespace
    pub namespace: String,

    /// Custom registry headers
    pub headers: HashMap<String, String>,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            url: "docker.io".to_string(),
            username: None,
            password: None,
            use_tls: true,
            namespace: "library".to_string(),
            headers: HashMap::new(),
        }
    }
}

impl RegistryConfig {
    /// Validate registry configuration
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        if self.url.is_empty() {
            return Err(DeploymentEnvironmentError::InvalidRegistry("Registry URL cannot be empty".to_string()));
        }

        if self.namespace.is_empty() {
            return Err(DeploymentEnvironmentError::InvalidRegistry("Registry namespace cannot be empty".to_string()));
        }

        Ok(())
    }
}

/// Image tagging strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaggingStrategy {
    Latest,
    CommitHash,
    Timestamp,
    SemanticVersion,
    BranchName,
}

/// Cleanup policy for old images
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPolicy {
    /// Maximum number of images to keep
    pub max_images: u32,

    /// Maximum age of images to keep (in days)
    pub max_age_days: u32,

    /// Whether to clean up untagged images
    pub clean_untagged: bool,

    /// Whether to clean up failed builds
    pub clean_failed_builds: bool,

    /// Cleanup schedule (cron expression)
    pub cleanup_schedule: String,
}

impl Default for CleanupPolicy {
    fn default() -> Self {
        Self {
            max_images: 10,
            max_age_days: 30,
            clean_untagged: true,
            clean_failed_builds: true,
            cleanup_schedule: "0 2 * * *".to_string(), // Daily at 2 AM
        }
    }
}

impl CleanupPolicy {
    /// Validate cleanup policy
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        if self.max_images == 0 {
            return Err(DeploymentEnvironmentError::InvalidCleanup("Max images must be greater than 0".to_string()));
        }

        if self.max_age_days == 0 {
            return Err(DeploymentEnvironmentError::InvalidCleanup("Max age days must be greater than 0".to_string()));
        }

        Ok(())
    }
}

/// Deployment strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    RollingUpdate,
    BlueGreen,
    Canary,
    Recreate,
}

/// Auto-scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalingConfig {
    /// Minimum number of instances
    pub min_instances: u32,

    /// Maximum number of instances
    pub max_instances: u32,

    /// Target CPU utilization percentage
    pub target_cpu_utilization: u8,

    /// Target memory utilization percentage
    pub target_memory_utilization: u8,

    /// Scale up cooldown period (in seconds)
    pub scale_up_cooldown: u32,

    /// Scale down cooldown period (in seconds)
    pub scale_down_cooldown: u32,

    /// Whether auto-scaling is enabled
    pub enabled: bool,
}

impl AutoScalingConfig {
    /// Validate auto-scaling configuration
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        if self.min_instances == 0 {
            return Err(DeploymentEnvironmentError::InvalidScaling("Min instances must be greater than 0".to_string()));
        }

        if self.max_instances <= self.min_instances {
            return Err(DeploymentEnvironmentError::InvalidScaling("Max instances must be greater than min instances".to_string()));
        }

        if self.target_cpu_utilization == 0 || self.target_cpu_utilization > 100 {
            return Err(DeploymentEnvironmentError::InvalidScaling("Target CPU utilization must be between 1-100%".to_string()));
        }

        if self.target_memory_utilization == 0 || self.target_memory_utilization > 100 {
            return Err(DeploymentEnvironmentError::InvalidScaling("Target memory utilization must be between 1-100%".to_string()));
        }

        Ok(())
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Whether monitoring is enabled
    pub enabled: bool,

    /// Metrics collection interval (in seconds)
    pub metrics_interval: u32,

    /// Monitoring endpoints
    pub endpoints: Vec<String>,

    /// Alert configuration
    pub alerts: AlertConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Prometheus integration
    pub prometheus: PrometheusConfig,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: 30,
            endpoints: vec!["/metrics".to_string(), "/health".to_string()],
            alerts: AlertConfig::default(),
            logging: LoggingConfig::default(),
            prometheus: PrometheusConfig::default(),
        }
    }
}

impl MonitoringConfig {
    /// Validate monitoring configuration
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        if self.metrics_interval == 0 {
            return Err(DeploymentEnvironmentError::InvalidMonitoring("Metrics interval must be greater than 0".to_string()));
        }

        if self.metrics_interval > 3600 {
            return Err(DeploymentEnvironmentError::InvalidMonitoring("Metrics interval must be less than 3600 seconds".to_string()));
        }

        // Validate alert configuration
        self.alerts.validate()?;

        // Validate logging configuration
        self.logging.validate()?;

        Ok(())
    }
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Whether alerts are enabled
    pub enabled: bool,

    /// Alert channels
    pub channels: Vec<AlertChannel>,

    /// Alert rules
    pub rules: Vec<AlertRule>,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            channels: Vec::new(),
            rules: Vec::new(),
        }
    }
}

impl AlertConfig {
    /// Validate alert configuration
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        for channel in &self.channels {
            channel.validate()?;
        }

        for rule in &self.rules {
            rule.validate()?;
        }

        Ok(())
    }
}

/// Alert channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertChannel {
    /// Channel name
    pub name: String,

    /// Channel type (email, slack, webhook, etc.)
    pub channel_type: AlertChannelType,

    /// Channel configuration
    pub configuration: HashMap<String, String>,
}

impl AlertChannel {
    /// Validate alert channel
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        if self.name.is_empty() {
            return Err(DeploymentEnvironmentError::InvalidAlert("Channel name cannot be empty".to_string()));
        }

        // Validate channel type specific configuration
        match self.channel_type {
            AlertChannelType::Email => {
                if !self.configuration.contains_key("to") {
                    return Err(DeploymentEnvironmentError::InvalidAlert("Email channel requires 'to' configuration".to_string()));
                }
            }
            AlertChannelType::Slack => {
                if !self.configuration.contains_key("webhook_url") {
                    return Err(DeploymentEnvironmentError::InvalidAlert("Slack channel requires 'webhook_url' configuration".to_string()));
                }
            }
            AlertChannelType::PagerDuty => {
                if !self.configuration.contains_key("service_key") {
                    return Err(DeploymentEnvironmentError::InvalidAlert("PagerDuty channel requires 'service_key' configuration".to_string()));
                }
            }
            AlertChannelType::Webhook => {
                if !self.configuration.contains_key("url") {
                    return Err(DeploymentEnvironmentError::InvalidAlert("Webhook channel requires 'url' configuration".to_string()));
                }
            }
        }

        Ok(())
    }
}

/// Alert channel types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertChannelType {
    Email,
    Slack,
    Webhook,
    PagerDuty,
}

/// Alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,

    /// Rule condition
    pub condition: String,

    /// Rule threshold
    pub threshold: f64,

    /// Rule duration (in seconds)
    pub duration: u32,

    /// Rule severity
    pub severity: AlertSeverity,

    /// Whether the rule is enabled
    pub enabled: bool,
}

impl AlertRule {
    /// Validate alert rule
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        if self.name.is_empty() {
            return Err(DeploymentEnvironmentError::InvalidAlert("Rule name cannot be empty".to_string()));
        }

        if self.condition.is_empty() {
            return Err(DeploymentEnvironmentError::InvalidAlert("Rule condition cannot be empty".to_string()));
        }

        if self.duration == 0 {
            return Err(DeploymentEnvironmentError::InvalidAlert("Rule duration must be greater than 0".to_string()));
        }

        Ok(())
    }
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: LogLevel,

    /// Log format
    pub format: LogFormat,

    /// Log output destination
    pub output: LogOutput,

    /// Log retention period (in days)
    pub retention_days: u32,

    /// Whether to enable structured logging
    pub structured: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Json,
            output: LogOutput::Stdout,
            retention_days: 30,
            structured: true,
        }
    }
}

impl LoggingConfig {
    /// Validate logging configuration
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        if self.retention_days == 0 {
            return Err(DeploymentEnvironmentError::InvalidLogging("Retention days must be greater than 0".to_string()));
        }

        Ok(())
    }
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Log formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Text,
    Json,
    Structured,
}

/// Log output destinations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    Stdout,
    Stderr,
    File(String),
    Syslog,
}

/// Prometheus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    /// Whether Prometheus integration is enabled
    pub enabled: bool,

    /// Prometheus metrics endpoint
    pub endpoint: String,

    /// Custom metrics
    pub custom_metrics: Vec<String>,

    /// Scrape interval (in seconds)
    pub scrape_interval: u32,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "/metrics".to_string(),
            custom_metrics: Vec::new(),
            scrape_interval: 30,
        }
    }
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Whether backups are enabled
    pub enabled: bool,

    /// Backup schedule (cron expression)
    pub schedule: String,

    /// Backup retention period (in days)
    pub retention_days: u32,

    /// Backup storage location
    pub storage_location: String,

    /// Backup encryption key (optional)
    pub encryption_key: Option<String>,

    /// Whether to compress backups
    pub compress: bool,
}

impl BackupConfig {
    /// Validate backup configuration
    pub fn validate(&self) -> Result<(), DeploymentEnvironmentError> {
        if self.enabled {
            if self.schedule.is_empty() {
                return Err(DeploymentEnvironmentError::InvalidBackup("Backup schedule cannot be empty".to_string()));
            }

            if self.storage_location.is_empty() {
                return Err(DeploymentEnvironmentError::InvalidBackup("Backup storage location cannot be empty".to_string()));
            }

            if self.retention_days == 0 {
                return Err(DeploymentEnvironmentError::InvalidBackup("Backup retention days must be greater than 0".to_string()));
            }
        }

        Ok(())
    }
}

/// Compliance requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequirement {
    /// Requirement name
    pub name: String,

    /// Requirement description
    pub description: String,

    /// Whether the requirement is met
    pub met: bool,

    /// Last verification timestamp
    pub last_verified: Option<chrono::DateTime<chrono::Utc>>,

    /// Verification method
    pub verification_method: String,
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    NotDeployed,
    Deploying,
    Deployed,
    Failed,
    Stopping,
    Stopped,
}

impl fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeploymentStatus::NotDeployed => write!(f, "Not Deployed"),
            DeploymentStatus::Deploying => write!(f, "Deploying"),
            DeploymentStatus::Deployed => write!(f, "Deployed"),
            DeploymentStatus::Failed => write!(f, "Failed"),
            DeploymentStatus::Stopping => write!(f, "Stopping"),
            DeploymentStatus::Stopped => write!(f, "Stopped"),
        }
    }
}

/// Errors related to deployment environments
#[derive(Debug, thiserror::Error)]
pub enum DeploymentEnvironmentError {
    #[error("Invalid name: {0}")]
    InvalidName(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Invalid registry: {0}")]
    InvalidRegistry(String),

    #[error("Invalid cleanup: {0}")]
    InvalidCleanup(String),

    #[error("Invalid scaling: {0}")]
    InvalidScaling(String),

    #[error("Invalid monitoring: {0}")]
    InvalidMonitoring(String),

    #[error("Invalid alert: {0}")]
    InvalidAlert(String),

    #[error("Invalid logging: {0}")]
    InvalidLogging(String),

    #[error("Invalid backup: {0}")]
    InvalidBackup(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_environment() {
        let env = DeploymentEnvironment::default();
        assert!(env.name.is_empty());
        assert_eq!(env.environment_type, EnvironmentType::Development);
        assert_eq!(env.deployment_status, DeploymentStatus::NotDeployed);
    }

    #[test]
    fn test_new_environment() {
        let env = DeploymentEnvironment::new("test-env".to_string(), EnvironmentType::Production);
        assert_eq!(env.name, "test-env");
        assert_eq!(env.environment_type, EnvironmentType::Production);
    }

    #[test]
    fn test_with_description() {
        let env = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Development)
            .with_description("Test environment".to_string());
        assert_eq!(env.description, "Test environment");
    }

    #[test]
    fn test_add_environment_variable() {
        let env = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Development)
            .add_environment_variable("TEST_VAR".to_string(), "test_value".to_string());
        assert_eq!(env.environment_variables.get("TEST_VAR"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_validate_valid_environment() {
        let env = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Development)
            .with_resource_limits(ResourceLimits::minimal());
        assert!(env.validate().is_ok());
    }

    #[test]
    fn test_validate_production_requirements() {
        let env = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Production);
        assert!(matches!(env.validate(), Err(DeploymentEnvironmentError::InvalidConfiguration(_))));
    }

    #[test]
    fn test_is_ready_for_deployment() {
        let ready = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Development)
            .with_resource_limits(ResourceLimits::minimal());

        let not_ready = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Development);

        assert!(ready.is_ready_for_deployment());
        assert!(!not_ready.is_ready_for_deployment());
    }

    #[test]
    fn test_docker_run_args() {
        let env = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Development)
            .add_environment_variable("TEST_VAR".to_string(), "test_value".to_string())
            .with_resource_limits(ResourceLimits::minimal());

        let args = env.docker_run_args().unwrap();
        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"TEST_VAR=test_value".to_string()));
        assert!(args.contains(&"--memory".to_string()));
    }

    #[test]
    fn test_health_score() {
        let deployed = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Development);
        let deployed_with_limits = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Development)
            .with_resource_limits(ResourceLimits::minimal())
            .with_security_profile(SecurityProfile::standard());

        assert!(deployed_with_limits.health_score() > deployed.health_score());
    }

    #[test]
    fn test_adapt_for_environment() {
        let original = DeploymentEnvironment::new("test".to_string(), EnvironmentType::Development);
        let adapted = original.adapt_for_environment(EnvironmentType::Production);

        assert_eq!(adapted.environment_type, EnvironmentType::Production);
        assert_ne!(adapted.id, original.id);
        assert!(adapted.resource_limits.is_some());
        assert!(adapted.security_profile.is_some());
    }

    #[test]
    fn test_display() {
        let env = DeploymentEnvironment::new("test-env".to_string(), EnvironmentType::Production);
        let display = format!("{}", env);
        assert!(display.contains("name=test-env"));
        assert!(display.contains("type=Production"));
        assert!(display.contains("status=Not Deployed"));
    }
}