use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// Health check configuration for Docker containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check command to execute
    pub command: String,

    /// Interval between health checks in seconds
    pub interval_seconds: u32,

    /// Timeout for health check in seconds
    pub timeout_seconds: u32,

    /// Number of consecutive failures needed to consider container unhealthy
    pub retries: u32,

    /// Start period for initialization in seconds
    pub start_period_seconds: u32,

    /// Health check thresholds
    pub thresholds: HealthCheckThresholds,

    /// Custom health check parameters
    pub custom_parameters: std::collections::HashMap<String, String>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            command: "curl -f http://localhost:4242/health || exit 1".to_string(),
            interval_seconds: 30,
            timeout_seconds: 10,
            retries: 3,
            start_period_seconds: 5,
            thresholds: HealthCheckThresholds::default(),
            custom_parameters: std::collections::HashMap::new(),
        }
    }
}

impl HealthCheckConfig {
    /// Create a new health check with custom command
    pub fn new(command: String) -> Self {
        Self {
            command,
            ..Default::default()
        }
    }

    /// Create an HTTP health check
    pub fn http_check(port: u16, path: String) -> Self {
        Self::new(format!(
            "curl -f http://localhost:{}{} || exit 1",
            port, path
        ))
    }

    /// Create a TCP health check
    pub fn tcp_check(port: u16) -> Self {
        Self::new(format!("nc -z localhost {} || exit 1", port))
    }

    /// Create a command health check
    pub fn command_check(command: String) -> Self {
        Self::new(command)
    }

    /// Set interval
    pub fn with_interval(mut self, seconds: u32) -> Self {
        self.interval_seconds = seconds;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, seconds: u32) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set retries
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Set start period
    pub fn with_start_period(mut self, seconds: u32) -> Self {
        self.start_period_seconds = seconds;
        self
    }

    /// Add custom parameter
    pub fn with_custom_parameter(mut self, key: String, value: String) -> Self {
        self.custom_parameters.insert(key, value);
        self
    }

    /// Validate the health check configuration
    pub fn validate(&self) -> Result<(), HealthCheckError> {
        if self.command.is_empty() {
            return Err(HealthCheckError::InvalidCommand(
                "Health check command cannot be empty".to_string(),
            ));
        }

        if self.interval_seconds == 0 {
            return Err(HealthCheckError::InvalidInterval(
                "Interval must be greater than 0".to_string(),
            ));
        }

        if self.interval_seconds > 3600 {
            return Err(HealthCheckError::InvalidInterval(
                "Interval must be less than 3600 seconds".to_string(),
            ));
        }

        if self.timeout_seconds == 0 {
            return Err(HealthCheckError::InvalidTimeout(
                "Timeout must be greater than 0".to_string(),
            ));
        }

        if self.timeout_seconds > 300 {
            return Err(HealthCheckError::InvalidTimeout(
                "Timeout must be less than 300 seconds".to_string(),
            ));
        }

        if self.retries == 0 {
            return Err(HealthCheckError::InvalidRetries(
                "Retries must be greater than 0".to_string(),
            ));
        }

        if self.retries > 10 {
            return Err(HealthCheckError::InvalidRetries(
                "Retries must be less than 10".to_string(),
            ));
        }

        if self.start_period_seconds > 600 {
            return Err(HealthCheckError::InvalidStartPeriod(
                "Start period must be less than 600 seconds".to_string(),
            ));
        }

        // Validate command doesn't contain dangerous operations
        if self.is_dangerous_command() {
            return Err(HealthCheckError::SecurityViolation(
                "Health check command contains potentially dangerous operations".to_string(),
            ));
        }

        // Validate thresholds
        self.thresholds.validate()?;

        Ok(())
    }

    /// Check if command contains potentially dangerous operations
    fn is_dangerous_command(&self) -> bool {
        let command_lower = self.command.to_lowercase();

        // Dangerous commands that could compromise container security
        command_lower.contains("rm -rf")
            || command_lower.contains("chmod 777")
            || command_lower.contains("chown root")
            || command_lower.contains("sudo")
            || command_lower.contains("su -")
            || command_lower.contains("/bin/bash")
            || command_lower.contains("/bin/sh")
            || command_lower.contains("wget ")
            || command_lower.contains("curl ") && command_lower.contains(" -o ")
            || command_lower.contains("exec")
            || command_lower.contains("eval")
            || command_lower.contains("system(")
    }

    /// Generate Docker health check arguments
    pub fn docker_args(&self) -> Result<Vec<String>, HealthCheckError> {
        self.validate()?;

        let mut args = Vec::new();

        // Health check command
        args.push("--health-cmd".to_string());
        args.push(self.command.clone());

        // Interval
        args.push("--health-interval".to_string());
        args.push(format!("{}s", self.interval_seconds));

        // Timeout
        args.push("--health-timeout".to_string());
        args.push(format!("{}s", self.timeout_seconds));

        // Retries
        args.push("--health-retries".to_string());
        args.push(format!("{}", self.retries));

        // Start period (if greater than 0)
        if self.start_period_seconds > 0 {
            args.push("--health-start-period".to_string());
            args.push(format!("{}s", self.start_period_seconds));
        }

        // Add custom parameters
        for (key, value) in &self.custom_parameters {
            args.push(format!("--health-{}", key));
            args.push(value.clone());
        }

        Ok(args)
    }

    /// Get the interval as a Duration
    pub fn interval_duration(&self) -> Duration {
        Duration::from_secs(self.interval_seconds as u64)
    }

    /// Get the timeout as a Duration
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds as u64)
    }

    /// Get the start period as a Duration
    pub fn start_period_duration(&self) -> Duration {
        Duration::from_secs(self.start_period_seconds as u64)
    }

    /// Calculate maximum time before container is considered unhealthy
    pub fn max_unhealthy_time(&self) -> Duration {
        self.interval_duration() * self.retries + self.timeout_duration()
    }

    /// Check if this is an HTTP health check
    pub fn is_http_check(&self) -> bool {
        self.command.to_lowercase().contains("curl") && self.command.contains("http")
    }

    /// Check if this is a TCP health check
    pub fn is_tcp_check(&self) -> bool {
        self.command.contains("nc -z") || self.command.contains("telnet")
    }

    /// Check if this is a command-based health check
    pub fn is_command_check(&self) -> bool {
        !self.is_http_check() && !self.is_tcp_check()
    }

    /// Calculate health check reliability score (0-100)
    pub fn reliability_score(&self) -> u8 {
        let mut score = 100;

        // Penalize long intervals
        if self.interval_seconds > 60 {
            score -= 10;
        } else if self.interval_seconds > 30 {
            score -= 5;
        }

        // Penalize short timeouts
        if self.timeout_seconds < 5 {
            score -= 15;
        } else if self.timeout_seconds < 10 {
            score -= 5;
        }

        // Reward appropriate retries
        if self.retries >= 3 && self.retries <= 5 {
            score += 10;
        } else if self.retries > 5 {
            score -= 5;
        }

        // Reward appropriate start period
        if self.start_period_seconds >= 5 && self.start_period_seconds <= 30 {
            score += 5;
        }

        // Reward HTTP checks (more reliable)
        if self.is_http_check() {
            score += 10;
        }

        // Reward TCP checks (reliable)
        if self.is_tcp_check() {
            score += 5;
        }

        // Penalize command checks (less reliable)
        if self.is_command_check() {
            score -= 5;
        }

        score.max(0).min(100)
    }

    /// Clone with modified parameters for different environments
    pub fn for_environment(&self, environment: &str) -> Self {
        let mut config = self.clone();

        match environment.to_lowercase().as_str() {
            "development" => {
                // More frequent checks in development
                config.interval_seconds = std::cmp::max(10, config.interval_seconds / 2);
                config.timeout_seconds = std::cmp::max(5, config.timeout_seconds / 2);
                config.retries = std::cmp::max(2, config.retries - 1);
            }
            "production" => {
                // Less frequent but more reliable checks in production
                config.interval_seconds = std::cmp::min(60, config.interval_seconds * 2);
                config.retries = std::cmp::min(5, config.retries + 1);
                config.start_period_seconds = std::cmp::min(30, config.start_period_seconds + 5);
            }
            "staging" => {
                // Balanced settings for staging
                config.interval_seconds =
                    std::cmp::max(15, std::cmp::min(45, config.interval_seconds));
            }
            _ => {}
        }

        config
    }
}

impl fmt::Display for HealthCheckConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HealthCheckConfig(interval={}s, timeout={}s, retries={}, type={}, reliability={})",
            self.interval_seconds,
            self.timeout_seconds,
            self.retries,
            if self.is_http_check() {
                "HTTP"
            } else if self.is_tcp_check() {
                "TCP"
            } else {
                "Command"
            },
            self.reliability_score()
        )
    }
}

/// Health check thresholds for different metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckThresholds {
    /// Response time threshold in milliseconds
    pub response_time_ms: u32,

    /// Memory usage threshold in percentage
    pub memory_usage_percent: u8,

    /// CPU usage threshold in percentage
    pub cpu_usage_percent: u8,

    /// Success rate threshold in percentage
    pub success_rate_percent: u8,

    /// Concurrent connections threshold
    pub concurrent_connections: u32,
}

impl Default for HealthCheckThresholds {
    fn default() -> Self {
        Self {
            response_time_ms: 1000,
            memory_usage_percent: 80,
            cpu_usage_percent: 70,
            success_rate_percent: 95,
            concurrent_connections: 1000,
        }
    }
}

impl HealthCheckThresholds {
    /// Create new thresholds with custom values
    pub fn new(response_time_ms: u32, memory_usage_percent: u8, cpu_usage_percent: u8) -> Self {
        Self {
            response_time_ms,
            memory_usage_percent,
            cpu_usage_percent,
            ..Default::default()
        }
    }

    /// Set response time threshold
    pub fn with_response_time(mut self, ms: u32) -> Self {
        self.response_time_ms = ms;
        self
    }

    /// Set memory usage threshold
    pub fn with_memory_usage(mut self, percent: u8) -> Self {
        self.memory_usage_percent = percent;
        self
    }

    /// Set CPU usage threshold
    pub fn with_cpu_usage(mut self, percent: u8) -> Self {
        self.cpu_usage_percent = percent;
        self
    }

    /// Set success rate threshold
    pub fn with_success_rate(mut self, percent: u8) -> Self {
        self.success_rate_percent = percent;
        self
    }

    /// Set concurrent connections threshold
    pub fn with_concurrent_connections(mut self, connections: u32) -> Self {
        self.concurrent_connections = connections;
        self
    }

    /// Validate thresholds
    pub fn validate(&self) -> Result<(), HealthCheckError> {
        if self.response_time_ms == 0 {
            return Err(HealthCheckError::InvalidThreshold(
                "Response time threshold must be greater than 0".to_string(),
            ));
        }

        if self.response_time_ms > 30000 {
            return Err(HealthCheckError::InvalidThreshold(
                "Response time threshold must be less than 30 seconds".to_string(),
            ));
        }

        if self.memory_usage_percent == 0 || self.memory_usage_percent > 100 {
            return Err(HealthCheckError::InvalidThreshold(
                "Memory usage threshold must be between 1-100%".to_string(),
            ));
        }

        if self.cpu_usage_percent == 0 || self.cpu_usage_percent > 100 {
            return Err(HealthCheckError::InvalidThreshold(
                "CPU usage threshold must be between 1-100%".to_string(),
            ));
        }

        if self.success_rate_percent == 0 || self.success_rate_percent > 100 {
            return Err(HealthCheckError::InvalidThreshold(
                "Success rate threshold must be between 1-100%".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if response time is acceptable
    pub fn is_response_time_acceptable(&self, response_time_ms: u32) -> bool {
        response_time_ms <= self.response_time_ms
    }

    /// Check if memory usage is acceptable
    pub fn is_memory_usage_acceptable(&self, memory_usage_percent: u8) -> bool {
        memory_usage_percent <= self.memory_usage_percent
    }

    /// Check if CPU usage is acceptable
    pub fn is_cpu_usage_acceptable(&self, cpu_usage_percent: u8) -> bool {
        cpu_usage_percent <= self.cpu_usage_percent
    }

    /// Check if success rate is acceptable
    pub fn is_success_rate_acceptable(&self, success_rate_percent: u8) -> bool {
        success_rate_percent >= self.success_rate_percent
    }

    /// Check if concurrent connections are acceptable
    pub fn is_concurrent_connections_acceptable(&self, connections: u32) -> bool {
        connections <= self.concurrent_connections
    }

    /// Calculate overall threshold compliance score (0-100)
    pub fn compliance_score(
        &self,
        response_time_ms: u32,
        memory_usage_percent: u8,
        cpu_usage_percent: u8,
    ) -> u8 {
        let mut score: u8 = 100;

        if !self.is_response_time_acceptable(response_time_ms) {
            let overage = response_time_ms.saturating_sub(self.response_time_ms);
            let penalty: u8 = (overage as f64 / self.response_time_ms as f64 * 20.0) as u8;
            score = score.saturating_sub(penalty);
        }

        if !self.is_memory_usage_acceptable(memory_usage_percent) {
            let overage = memory_usage_percent.saturating_sub(self.memory_usage_percent);
            let penalty: u8 = (overage as f64 / self.memory_usage_percent as f64 * 20.0) as u8;
            score = score.saturating_sub(penalty);
        }

        if !self.is_cpu_usage_acceptable(cpu_usage_percent) {
            let overage = cpu_usage_percent.saturating_sub(self.cpu_usage_percent);
            let penalty: u8 = (overage as f64 / self.cpu_usage_percent as f64 * 20.0) as u8;
            score = score.saturating_sub(penalty);
        }

        score.max(0).min(100)
    }
}

/// Errors related to health check configuration
#[derive(Debug, thiserror::Error)]
pub enum HealthCheckError {
    #[error("Invalid health check command: {0}")]
    InvalidCommand(String),

    #[error("Invalid interval: {0}")]
    InvalidInterval(String),

    #[error("Invalid timeout: {0}")]
    InvalidTimeout(String),

    #[error("Invalid retries: {0}")]
    InvalidRetries(String),

    #[error("Invalid start period: {0}")]
    InvalidStartPeriod(String),

    #[error("Invalid threshold: {0}")]
    InvalidThreshold(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_health_check() {
        let health = HealthCheckConfig::default();
        assert!(health.command.contains("curl"));
        assert_eq!(health.interval_seconds, 30);
        assert_eq!(health.timeout_seconds, 10);
        assert_eq!(health.retries, 3);
    }

    #[test]
    fn test_http_check() {
        let health = HealthCheckConfig::http_check(8080, "/health".to_string());
        assert!(health.command.contains("curl"));
        assert!(health.command.contains("localhost:8080"));
        assert!(health.command.contains("/health"));
    }

    #[test]
    fn test_tcp_check() {
        let health = HealthCheckConfig::tcp_check(8080);
        assert!(health.command.contains("nc -z"));
        assert!(health.command.contains("localhost:8080"));
    }

    #[test]
    fn test_command_check() {
        let health = HealthCheckConfig::command_check("pg_isready".to_string());
        assert_eq!(health.command, "pg_isready");
    }

    #[test]
    fn test_with_interval() {
        let health = HealthCheckConfig::default().with_interval(60);
        assert_eq!(health.interval_seconds, 60);
    }

    #[test]
    fn test_validate_valid_config() {
        let health = HealthCheckConfig::http_check(8080, "/health".to_string());
        assert!(health.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_command() {
        let health = HealthCheckConfig::new("".to_string());
        assert!(matches!(
            health.validate(),
            Err(HealthCheckError::InvalidCommand(_))
        ));
    }

    #[test]
    fn test_validate_zero_interval() {
        let mut health = HealthCheckConfig::default();
        health.interval_seconds = 0;
        assert!(matches!(
            health.validate(),
            Err(HealthCheckError::InvalidInterval(_))
        ));
    }

    #[test]
    fn test_validate_dangerous_command() {
        let health = HealthCheckConfig::new("rm -rf /".to_string());
        assert!(matches!(
            health.validate(),
            Err(HealthCheckError::SecurityViolation(_))
        ));
    }

    #[test]
    fn test_docker_args() {
        let health = HealthCheckConfig::http_check(8080, "/health".to_string())
            .with_interval(15)
            .with_timeout(5)
            .with_retries(2);

        let args = health.docker_args().unwrap();
        assert!(args.contains(&"--health-cmd".to_string()));
        assert!(args.contains(&"--health-interval".to_string()));
        assert!(args.contains(&"15s".to_string()));
        assert!(args.contains(&"--health-timeout".to_string()));
        assert!(args.contains(&"5s".to_string()));
        assert!(args.contains(&"--health-retries".to_string()));
        assert!(args.contains(&"2".to_string()));
    }

    #[test]
    fn test_check_types() {
        let http_health = HealthCheckConfig::http_check(8080, "/health".to_string());
        let tcp_health = HealthCheckConfig::tcp_check(8080);
        let command_health = HealthCheckConfig::command_check("echo 'ok'".to_string());

        assert!(http_health.is_http_check());
        assert!(!http_health.is_tcp_check());
        assert!(!http_health.is_command_check());

        assert!(!tcp_health.is_http_check());
        assert!(tcp_health.is_tcp_check());
        assert!(!tcp_health.is_command_check());

        assert!(!command_health.is_http_check());
        assert!(!command_health.is_tcp_check());
        assert!(command_health.is_command_check());
    }

    #[test]
    fn test_reliability_score() {
        let reliable = HealthCheckConfig::http_check(8080, "/health".to_string())
            .with_interval(30)
            .with_timeout(10)
            .with_retries(3);

        let unreliable = HealthCheckConfig::command_check("slow_command".to_string())
            .with_interval(120)
            .with_timeout(3)
            .with_retries(1);

        assert!(reliable.reliability_score() > unreliable.reliability_score());
    }

    #[test]
    fn test_environment_adaptation() {
        let base = HealthCheckConfig::http_check(8080, "/health".to_string())
            .with_interval(30)
            .with_timeout(10)
            .with_retries(3);

        let dev = base.for_environment("development");
        let prod = base.for_environment("production");

        assert!(dev.interval_seconds < base.interval_seconds);
        assert!(dev.timeout_seconds <= base.timeout_seconds);
        assert!(prod.interval_seconds >= base.interval_seconds);
        assert!(prod.retries >= base.retries);
    }

    #[test]
    fn test_thresholds() {
        let thresholds = HealthCheckThresholds::new(1000, 80, 70)
            .with_success_rate(95)
            .with_concurrent_connections(1000);

        assert!(thresholds.is_response_time_acceptable(800));
        assert!(!thresholds.is_response_time_acceptable(1500));

        assert!(thresholds.is_memory_usage_acceptable(70));
        assert!(!thresholds.is_memory_usage_acceptable(90));

        assert_eq!(thresholds.compliance_score(500, 60, 50), 100);
        assert!(thresholds.compliance_score(2000, 90, 80) < 100);
    }

    #[test]
    fn test_display() {
        let health = HealthCheckConfig::http_check(8080, "/health".to_string());
        let display = format!("{}", health);
        assert!(display.contains("interval=30s"));
        assert!(display.contains("timeout=10s"));
        assert!(display.contains("type=HTTP"));
    }
}
