use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

/// Security scanning configuration for Docker containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanConfig {
    /// Unique identifier for this scan configuration
    pub id: uuid::Uuid,

    /// Name of the scan configuration
    pub name: String,

    /// Description of the scan configuration
    pub description: String,

    /// Types of scans to perform
    pub scan_types: HashSet<ScanType>,

    /// Image name to scan
    pub image_name: String,

    /// Container name to scan (if scanning running container)
    pub container_name: Option<String>,

    /// Scan schedule configuration
    pub schedule: ScanSchedule,

    /// Severity thresholds for failing the scan
    pub severity_thresholds: SeverityThresholds,

    /// Compliance standards to check
    pub compliance_standards: HashSet<ComplianceStandard>,

    /// Output format for scan results
    pub output_format: OutputFormat,

    /// Notification configuration
    pub notifications: NotificationConfig,

    /// Custom scan parameters
    pub custom_parameters: std::collections::HashMap<String, String>,

    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for SecurityScanConfig {
    fn default() -> Self {
        let mut scan_types = HashSet::new();
        scan_types.insert(ScanType::Vulnerability);
        scan_types.insert(ScanType::Configuration);

        let mut compliance_standards = HashSet::new();
        compliance_standards.insert(ComplianceStandard::CISDockerBenchmark);
        compliance_standards.insert(ComplianceStandard::NIST800190);

        let now = chrono::Utc::now();

        Self {
            id: uuid::Uuid::new_v4(),
            name: String::new(),
            description: String::new(),
            scan_types,
            image_name: String::new(),
            container_name: None,
            schedule: ScanSchedule::default(),
            severity_thresholds: SeverityThresholds::default(),
            compliance_standards,
            output_format: OutputFormat::Json,
            notifications: NotificationConfig::default(),
            custom_parameters: std::collections::HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl SecurityScanConfig {
    /// Create a new security scan configuration
    pub fn new(name: String, image_name: String) -> Self {
        let mut config = Self::default();
        config.name = name;
        config.image_name = image_name;
        config
    }

    /// Create a builder for SecurityScanConfig
    pub fn builder() -> SecurityScanConfigBuilder {
        SecurityScanConfigBuilder::default()
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Add scan type
    pub fn add_scan_type(mut self, scan_type: ScanType) -> Self {
        self.scan_types.insert(scan_type);
        self
    }

    /// Remove scan type
    pub fn remove_scan_type(mut self, scan_type: ScanType) -> Self {
        self.scan_types.remove(&scan_type);
        self
    }

    /// Set container name for scanning
    pub fn with_container_name(mut self, container_name: String) -> Self {
        self.container_name = Some(container_name);
        self
    }

    /// Set scan schedule
    pub fn with_schedule(mut self, schedule: ScanSchedule) -> Self {
        self.schedule = schedule;
        self
    }

    /// Set severity thresholds
    pub fn with_severity_thresholds(mut self, thresholds: SeverityThresholds) -> Self {
        self.severity_thresholds = thresholds;
        self
    }

    /// Add compliance standard
    pub fn add_compliance_standard(mut self, standard: ComplianceStandard) -> Self {
        self.compliance_standards.insert(standard);
        self
    }

    /// Set output format
    pub fn with_output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Set notification configuration
    pub fn with_notifications(mut self, notifications: NotificationConfig) -> Self {
        self.notifications = notifications;
        self
    }

    /// Add custom parameter
    pub fn add_custom_parameter(mut self, key: String, value: String) -> Self {
        self.custom_parameters.insert(key, value);
        self
    }

    /// Validate the scan configuration
    pub fn validate(&self) -> Result<(), SecurityScanError> {
        if self.name.is_empty() {
            return Err(SecurityScanError::InvalidName(
                "Name cannot be empty".to_string(),
            ));
        }

        if self.image_name.is_empty() && self.container_name.is_none() {
            return Err(SecurityScanError::InvalidTarget(
                "Either image_name or container_name must be specified".to_string(),
            ));
        }

        if self.scan_types.is_empty() {
            return Err(SecurityScanError::InvalidConfiguration(
                "At least one scan type must be specified".to_string(),
            ));
        }

        // Validate schedule
        self.schedule.validate()?;

        // Validate severity thresholds
        self.severity_thresholds.validate()?;

        // Validate notification configuration
        self.notifications.validate()?;

        Ok(())
    }

    /// Check if this configuration should scan images
    pub fn scans_images(&self) -> bool {
        !self.image_name.is_empty()
    }

    /// Check if this configuration should scan running containers
    pub fn scans_containers(&self) -> bool {
        self.container_name.is_some()
    }

    /// Check if a specific scan type is enabled
    pub fn has_scan_type(&self, scan_type: ScanType) -> bool {
        self.scan_types.contains(&scan_type)
    }

    /// Check if a specific compliance standard is enabled
    pub fn has_compliance_standard(&self, standard: ComplianceStandard) -> bool {
        self.compliance_standards.contains(&standard)
    }

    /// Generate scan command based on configuration
    pub fn generate_scan_command(&self) -> Result<Vec<String>, SecurityScanError> {
        self.validate()?;

        let mut command = Vec::new();

        // Base command depends on scan type
        if self.has_scan_type(ScanType::Vulnerability) {
            command.extend(self.generate_trivy_command());
        }

        if self.has_scan_type(ScanType::Configuration) {
            command.extend(self.generate_hadolint_command());
        }

        if self.has_scan_type(ScanType::Compliance) {
            command.extend(self.generate_compliance_command());
        }

        Ok(command)
    }

    /// Generate Trivy command for vulnerability scanning
    fn generate_trivy_command(&self) -> Vec<String> {
        let mut cmd = vec!["trivy".to_string()];

        if self.scans_images() {
            cmd.push("image".to_string());
            cmd.push(self.image_name.clone());
        } else if let Some(ref container) = self.container_name {
            cmd.push("container".to_string());
            cmd.push(container.clone());
        }

        // Add severity threshold
        if self.severity_thresholds.fail_on_critical {
            cmd.push("--exit-code".to_string());
            cmd.push("1".to_string());
            cmd.push("--severity".to_string());
            cmd.push("CRITICAL".to_string());
        }

        // Add output format
        cmd.push("--format".to_string());
        cmd.push(match self.output_format {
            OutputFormat::Json => "json".to_string(),
            OutputFormat::Xml => "xml".to_string(),
            OutputFormat::Html => "template".to_string(),
            OutputFormat::Sarif => "sarif".to_string(),
            OutputFormat::Text => "table".to_string(),
        });

        cmd
    }

    /// Generate Hadolint command for Dockerfile scanning
    fn generate_hadolint_command(&self) -> Vec<String> {
        vec![
            "hadolint".to_string(),
            "docker/Dockerfile.hardened".to_string(),
        ]
    }

    /// Generate compliance scanning command
    fn generate_compliance_command(&self) -> Vec<String> {
        let mut cmd = vec!["docker".to_string(), "run".to_string(), "--rm".to_string()];

        // Mount Docker socket for container inspection
        cmd.push("-v".to_string());
        cmd.push("/var/run/docker.sock:/var/run/docker.sock".to_string());

        // Add Docker Bench Security image
        cmd.push("docker/docker-bench-security".to_string());

        // Add compliance standards as arguments
        for standard in &self.compliance_standards {
            match standard {
                ComplianceStandard::CISDockerBenchmark => {
                    cmd.push("-c".to_string());
                    cmd.push("cis".to_string());
                }
                ComplianceStandard::NIST800190 => {
                    cmd.push("-c".to_string());
                    cmd.push("nist".to_string());
                }
                ComplianceStandard::PCIDSS => {
                    cmd.push("-c".to_string());
                    cmd.push("pci".to_string());
                }
                ComplianceStandard::SOC2 => {
                    cmd.push("-c".to_string());
                    cmd.push("soc2".to_string());
                }
                ComplianceStandard::ISO27001 => {
                    cmd.push("-c".to_string());
                    cmd.push("iso27001".to_string());
                }
                ComplianceStandard::HIPAA => {
                    cmd.push("-c".to_string());
                    cmd.push("hipaa".to_string());
                }
                ComplianceStandard::GDPR => {
                    cmd.push("-c".to_string());
                    cmd.push("gdpr".to_string());
                }
            }
        }

        cmd
    }

    /// Calculate scan complexity score (0-100)
    pub fn complexity_score(&self) -> u8 {
        let mut score = 0;

        // Base score for scan types
        score += self.scan_types.len() as u8 * 20;

        // Compliance standards add complexity
        score += self.compliance_standards.len() as u8 * 10;

        // Schedule frequency adds complexity
        match self.schedule.frequency {
            ScheduleFrequency::Hourly => score += 5,
            ScheduleFrequency::Daily => score += 3,
            ScheduleFrequency::Weekly => score += 2,
            ScheduleFrequency::Monthly => score += 1,
        }

        // Notifications add complexity
        if !self.notifications.email_recipients.is_empty() {
            score += 5;
        }

        if !self.notifications.webhook_urls.is_empty() {
            score += 5;
        }

        // Custom parameters add complexity
        score += (self.custom_parameters.len() as u8) * 2;

        score.min(100)
    }

    /// Estimate scan duration based on configuration
    pub fn estimated_duration(&self) -> Duration {
        let mut base_seconds = 30;

        // Add time for each scan type
        if self.has_scan_type(ScanType::Vulnerability) {
            base_seconds += 60;
        }

        if self.has_scan_type(ScanType::Configuration) {
            base_seconds += 10;
        }

        if self.has_scan_type(ScanType::Compliance) {
            base_seconds += 120;
        }

        // Add time for compliance standards
        base_seconds += self.compliance_standards.len() as u64 * 30;

        // Consider image size (rough estimate)
        if !self.image_name.is_empty() {
            base_seconds += 60; // Assume average image size
        }

        Duration::from_secs(base_seconds)
    }
}

impl fmt::Display for SecurityScanConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SecurityScanConfig(name={}, scan_types={}, complexity={}, estimated_duration={:?})",
            self.name,
            self.scan_types.len(),
            self.complexity_score(),
            self.estimated_duration()
        )
    }
}

/// Types of security scans
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ScanType {
    /// Vulnerability scanning (CVEs, outdated packages)
    Vulnerability,
    /// Configuration scanning (Dockerfile best practices)
    Configuration,
    /// Compliance checking (CIS, NIST, etc.)
    Compliance,
    /// Runtime security monitoring
    Runtime,
    /// Malware scanning
    Malware,
    /// Secret scanning
    Secrets,
}

impl fmt::Display for ScanType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanType::Vulnerability => write!(f, "vulnerability"),
            ScanType::Configuration => write!(f, "configuration"),
            ScanType::Compliance => write!(f, "compliance"),
            ScanType::Runtime => write!(f, "runtime"),
            ScanType::Malware => write!(f, "malware"),
            ScanType::Secrets => write!(f, "secrets"),
        }
    }
}

/// Scan schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSchedule {
    /// When to run the scan
    pub frequency: ScheduleFrequency,

    /// Specific time to run (for daily/weekly schedules)
    pub run_time: Option<chrono::NaiveTime>,

    /// Day of week (for weekly schedules)
    pub day_of_week: Option<chrono::Weekday>,

    /// Day of month (for monthly schedules)
    pub day_of_month: Option<u32>,

    /// Whether the schedule is enabled
    pub enabled: bool,

    /// Timezone for the schedule
    pub timezone: String,
}

impl Default for ScanSchedule {
    fn default() -> Self {
        Self {
            frequency: ScheduleFrequency::Daily,
            run_time: Some(chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap()),
            day_of_week: None,
            day_of_month: None,
            enabled: true,
            timezone: "UTC".to_string(),
        }
    }
}

impl ScanSchedule {
    /// Validate the schedule
    pub fn validate(&self) -> Result<(), SecurityScanError> {
        match self.frequency {
            ScheduleFrequency::Weekly => {
                if self.day_of_week.is_none() {
                    return Err(SecurityScanError::InvalidSchedule(
                        "Day of week required for weekly schedule".to_string(),
                    ));
                }
            }
            ScheduleFrequency::Monthly => {
                if self.day_of_month.is_none() {
                    return Err(SecurityScanError::InvalidSchedule(
                        "Day of month required for monthly schedule".to_string(),
                    ));
                }
                if let Some(day) = self.day_of_month {
                    if day < 1 || day > 31 {
                        return Err(SecurityScanError::InvalidSchedule(
                            "Day of month must be between 1-31".to_string(),
                        ));
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Calculate next run time
    pub fn next_run_time(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        if !self.enabled {
            return None;
        }

        let now = chrono::Utc::now().naive_utc();
        let run_time = self
            .run_time
            .unwrap_or_else(|| chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());

        match self.frequency {
            ScheduleFrequency::Hourly => {
                let next = now.date().and_time(run_time);
                if next > now {
                    Some(chrono::DateTime::from_utc(next, chrono::Utc))
                } else {
                    Some(chrono::DateTime::from_utc(
                        now.date().and_time(run_time) + chrono::Duration::hours(1),
                        chrono::Utc,
                    ))
                }
            }
            ScheduleFrequency::Daily => {
                let next = now.date().and_time(run_time);
                if next > now {
                    Some(chrono::DateTime::from_utc(next, chrono::Utc))
                } else {
                    Some(chrono::DateTime::from_utc(
                        now.date().and_time(run_time) + chrono::Duration::days(1),
                        chrono::Utc,
                    ))
                }
            }
            ScheduleFrequency::Weekly => {
                let day = self.day_of_week.unwrap();
                let mut current = now.date();
                loop {
                    if current.weekday() == day {
                        let next = current.and_time(run_time);
                        if next > now {
                            return Some(chrono::DateTime::from_utc(next, chrono::Utc));
                        }
                    }
                    current = current.succ();
                }
            }
            ScheduleFrequency::Monthly => {
                let day = self.day_of_month.unwrap();
                let mut current = now.date();
                loop {
                    if current.day() == day {
                        let next = current.and_time(run_time);
                        if next > now {
                            return Some(chrono::DateTime::from_utc(next, chrono::Utc));
                        }
                    }
                    current = current.succ();
                    // Handle months with fewer days
                    if current.day() > day && current.month() % 12 == 1 {
                        current =
                            chrono::NaiveDate::from_ymd_opt(current.year(), current.month(), 1)
                                .unwrap();
                    }
                }
            }
        }
    }
}

/// Schedule frequency options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

/// Severity thresholds for scan results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityThresholds {
    /// Fail scan on critical vulnerabilities
    pub fail_on_critical: bool,

    /// Fail scan on high vulnerabilities
    pub fail_on_high: bool,

    /// Maximum allowed critical vulnerabilities
    pub max_critical: u32,

    /// Maximum allowed high vulnerabilities
    pub max_high: u32,

    /// Maximum allowed medium vulnerabilities
    pub max_medium: u32,

    /// Maximum allowed low vulnerabilities
    pub max_low: u32,

    /// Score threshold (0-100) for failing the scan
    pub minimum_score: u8,
}

impl Default for SeverityThresholds {
    fn default() -> Self {
        Self {
            fail_on_critical: true,
            fail_on_high: false,
            max_critical: 0,
            max_high: 5,
            max_medium: 20,
            max_low: 100,
            minimum_score: 80,
        }
    }
}

impl SeverityThresholds {
    /// Validate thresholds
    pub fn validate(&self) -> Result<(), SecurityScanError> {
        if self.minimum_score > 100 {
            return Err(SecurityScanError::InvalidThreshold(
                "Minimum score must be <= 100".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if scan results pass thresholds
    pub fn passes_thresholds(&self, results: &ScanResults) -> bool {
        // Check critical vulnerabilities
        if self.fail_on_critical && results.critical_count > self.max_critical {
            return false;
        }

        // Check high vulnerabilities
        if self.fail_on_high && results.high_count > self.max_high {
            return false;
        }

        // Check medium vulnerabilities
        if results.medium_count > self.max_medium {
            return false;
        }

        // Check low vulnerabilities
        if results.low_count > self.max_low {
            return false;
        }

        // Check minimum score
        if results.security_score < self.minimum_score {
            return false;
        }

        true
    }
}

/// Compliance standards
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComplianceStandard {
    CISDockerBenchmark,
    NIST800190,
    PCIDSS,
    SOC2,
    ISO27001,
    HIPAA,
    GDPR,
}

impl fmt::Display for ComplianceStandard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComplianceStandard::CISDockerBenchmark => write!(f, "CIS Docker Benchmark"),
            ComplianceStandard::NIST800190 => write!(f, "NIST 800-190"),
            ComplianceStandard::PCIDSS => write!(f, "PCI DSS"),
            ComplianceStandard::SOC2 => write!(f, "SOC 2"),
            ComplianceStandard::ISO27001 => write!(f, "ISO 27001"),
            ComplianceStandard::HIPAA => write!(f, "HIPAA"),
            ComplianceStandard::GDPR => write!(f, "GDPR"),
        }
    }
}

/// Output formats for scan results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Xml,
    Html,
    Sarif,
    Text,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Email recipients for notifications
    pub email_recipients: Vec<String>,

    /// Webhook URLs for notifications
    pub webhook_urls: Vec<String>,

    /// Slack webhook URLs
    pub slack_webhooks: Vec<String>,

    /// Whether to send notifications on scan success
    pub notify_on_success: bool,

    /// Whether to send notifications on scan failure
    pub notify_on_failure: bool,

    /// Notification template
    pub template: String,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            email_recipients: Vec::new(),
            webhook_urls: Vec::new(),
            slack_webhooks: Vec::new(),
            notify_on_success: false,
            notify_on_failure: true,
            template: "default".to_string(),
        }
    }
}

impl NotificationConfig {
    /// Validate notification configuration
    pub fn validate(&self) -> Result<(), SecurityScanError> {
        for email in &self.email_recipients {
            if !email.contains('@') {
                return Err(SecurityScanError::InvalidNotification(format!(
                    "Invalid email address: {}",
                    email
                )));
            }
        }

        for webhook in &self.webhook_urls {
            if !webhook.starts_with("http://") && !webhook.starts_with("https://") {
                return Err(SecurityScanError::InvalidNotification(format!(
                    "Invalid webhook URL: {}",
                    webhook
                )));
            }
        }

        Ok(())
    }

    /// Check if notifications are enabled
    pub fn notifications_enabled(&self) -> bool {
        !self.email_recipients.is_empty()
            || !self.webhook_urls.is_empty()
            || !self.slack_webhooks.is_empty()
    }
}

/// Scan results structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResults {
    /// Scan configuration ID
    pub scan_config_id: uuid::Uuid,

    /// Scan timestamp
    pub scan_timestamp: chrono::DateTime<chrono::Utc>,

    /// Image name that was scanned
    pub image_name: String,

    /// Container name that was scanned
    pub container_name: Option<String>,

    /// Vulnerability counts by severity
    pub critical_count: u32,
    pub high_count: u32,
    pub medium_count: u32,
    pub low_count: u32,

    /// Security score (0-100)
    pub security_score: u8,

    /// Compliance results
    pub compliance_results: std::collections::HashMap<ComplianceStandard, bool>,

    /// Scan duration
    pub scan_duration: Duration,

    /// Whether the scan passed all thresholds
    pub passed: bool,

    /// Detailed findings
    pub findings: Vec<String>,
}

impl Default for ScanResults {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            scan_config_id: uuid::Uuid::nil(),
            scan_timestamp: now,
            image_name: String::new(),
            container_name: None,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            security_score: 100,
            compliance_results: std::collections::HashMap::new(),
            scan_duration: Duration::from_secs(0),
            passed: true,
            findings: Vec::new(),
        }
    }
}

/// Errors related to security scanning
#[derive(Debug, thiserror::Error)]
pub enum SecurityScanError {
    #[error("Invalid name: {0}")]
    InvalidName(String),

    #[error("Invalid target: {0}")]
    InvalidTarget(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Invalid schedule: {0}")]
    InvalidSchedule(String),

    #[error("Invalid threshold: {0}")]
    InvalidThreshold(String),

    #[error("Invalid notification: {0}")]
    InvalidNotification(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scan_config() {
        let config = SecurityScanConfig::default();
        assert!(config.name.is_empty());
        assert!(config.image_name.is_empty());
        assert!(config.has_scan_type(ScanType::Vulnerability));
        assert!(config.has_scan_type(ScanType::Configuration));
    }

    #[test]
    fn test_new_scan_config() {
        let config =
            SecurityScanConfig::new("test-scan".to_string(), "test-image:latest".to_string());
        assert_eq!(config.name, "test-scan");
        assert_eq!(config.image_name, "test-image:latest");
    }

    #[test]
    fn test_scan_types() {
        let config = SecurityScanConfig::new("test".to_string(), "test:latest".to_string())
            .add_scan_type(ScanType::Compliance)
            .add_scan_type(ScanType::Malware);

        assert!(config.has_scan_type(ScanType::Compliance));
        assert!(config.has_scan_type(ScanType::Malware));
        assert!(!config.has_scan_type(ScanType::Secrets));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = SecurityScanConfig::new("test".to_string(), "test:latest".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let config = SecurityScanConfig::new("".to_string(), "test:latest".to_string());
        assert!(matches!(
            config.validate(),
            Err(SecurityScanError::InvalidName(_))
        ));
    }

    #[test]
    fn test_validate_no_target() {
        let config = SecurityScanConfig::new("test".to_string(), "".to_string());
        assert!(matches!(
            config.validate(),
            Err(SecurityScanError::InvalidTarget(_))
        ));
    }

    #[test]
    fn test_generate_trivy_command() {
        let config = SecurityScanConfig::new("test".to_string(), "test:latest".to_string());
        let cmd = config.generate_trivy_command();

        assert!(cmd.contains(&"trivy".to_string()));
        assert!(cmd.contains(&"image".to_string()));
        assert!(cmd.contains(&"test:latest".to_string()));
    }

    #[test]
    fn test_complexity_score() {
        let simple = SecurityScanConfig::new("simple".to_string(), "test:latest".to_string());
        let complex = SecurityScanConfig::new("complex".to_string(), "test:latest".to_string())
            .add_scan_type(ScanType::Compliance)
            .add_scan_type(ScanType::Malware)
            .add_compliance_standard(ComplianceStandard::CISDockerBenchmark)
            .add_compliance_standard(ComplianceStandard::NIST800190);

        assert!(complex.complexity_score() > simple.complexity_score());
    }

    #[test]
    fn test_estimated_duration() {
        let config = SecurityScanConfig::new("test".to_string(), "test:latest".to_string());
        let duration = config.estimated_duration();

        assert!(duration > Duration::from_secs(0));
        assert!(duration < Duration::from_secs(300)); // Should be reasonable
    }

    #[test]
    fn test_schedule_validation() {
        let mut schedule = ScanSchedule::default();
        schedule.frequency = ScheduleFrequency::Weekly;
        schedule.day_of_week = None;

        let config = SecurityScanConfig::new("test".to_string(), "test:latest".to_string())
            .with_schedule(schedule);

        assert!(matches!(
            config.validate(),
            Err(SecurityScanError::InvalidSchedule(_))
        ));
    }

    #[test]
    fn test_threshold_validation() {
        let mut thresholds = SeverityThresholds::default();
        thresholds.minimum_score = 150;

        let config = SecurityScanConfig::new("test".to_string(), "test:latest".to_string())
            .with_severity_thresholds(thresholds);

        assert!(matches!(
            config.validate(),
            Err(SecurityScanError::InvalidThreshold(_))
        ));
    }

    #[test]
    fn test_notification_validation() {
        let mut notifications = NotificationConfig::default();
        notifications
            .email_recipients
            .push("invalid-email".to_string());

        let config = SecurityScanConfig::new("test".to_string(), "test:latest".to_string())
            .with_notifications(notifications);

        assert!(matches!(
            config.validate(),
            Err(SecurityScanError::InvalidNotification(_))
        ));
    }

    #[test]
    fn test_display() {
        let config = SecurityScanConfig::new("test-scan".to_string(), "test:latest".to_string());
        let display = format!("{}", config);
        assert!(display.contains("name=test-scan"));
        assert!(display.contains("scan_types="));
    }
}

/// Builder for SecurityScanConfig
#[derive(Default)]
pub struct SecurityScanConfigBuilder {
    name: String,
    description: String,
    scan_types: HashSet<ScanType>,
    image_name: String,
    container_name: Option<String>,
    schedule: Option<ScanSchedule>,
    severity_thresholds: Option<SeverityThresholds>,
    compliance_standards: HashSet<ComplianceStandard>,
    output_format: Option<OutputFormat>,
    notifications: Option<NotificationConfig>,
    custom_parameters: std::collections::HashMap<String, String>,
    severity_threshold: Option<String>,
}

impl SecurityScanConfigBuilder {
    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn image_name(mut self, image_name: String) -> Self {
        self.image_name = image_name;
        self
    }

    pub fn description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn scan_types(mut self, scan_types: Vec<ScanType>) -> Self {
        self.scan_types = scan_types.into_iter().collect();
        self
    }

    pub fn severity_threshold(mut self, threshold: String) -> Self {
        self.severity_threshold = Some(threshold);
        self
    }

    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = Some(format);
        self
    }

    pub fn build(self) -> Result<SecurityScanConfig, SecurityScanError> {
        let now = chrono::Utc::now();
        Ok(SecurityScanConfig {
            id: uuid::Uuid::new_v4(),
            name: self.name,
            description: self.description,
            scan_types: if self.scan_types.is_empty() {
                let mut types = HashSet::new();
                types.insert(ScanType::Vulnerability);
                types
            } else {
                self.scan_types
            },
            image_name: self.image_name,
            container_name: self.container_name,
            schedule: self.schedule.unwrap_or_default(),
            severity_thresholds: self.severity_thresholds.unwrap_or_default(),
            compliance_standards: if self.compliance_standards.is_empty() {
                let mut standards = HashSet::new();
                standards.insert(ComplianceStandard::CISDockerBenchmark);
                standards
            } else {
                self.compliance_standards
            },
            output_format: self.output_format.unwrap_or(OutputFormat::Json),
            notifications: self.notifications.unwrap_or_default(),
            custom_parameters: self.custom_parameters,
            created_at: now,
            updated_at: now,
        })
    }
}
