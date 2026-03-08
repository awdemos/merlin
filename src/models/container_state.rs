use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Container status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerStatus {
    Created,
    Running,
    Stopped,
    Paused,
    Exited,
    Failed,
    Restarting,
    Removing,
    Dead,
}

/// Container metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetrics {
    pub container_id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub memory_limit_mb: u64,
    pub memory_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub block_read_bytes: u64,
    pub block_write_bytes: u64,
    pub pids_current: u32,
    pub pids_limit: u32,
    pub restart_count: u32,
    pub uptime_seconds: u64,
    pub health_status: Option<String>,
}

/// Container event for auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerEvent {
    pub id: uuid::Uuid,
    pub container_id: uuid::Uuid,
    pub event_type: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub details: HashMap<String, String>,
}

/// Container state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerState {
    pub id: uuid::Uuid,
    pub config_id: uuid::Uuid,
    pub image_id: String,
    pub status: ContainerStatus,
    pub health_status: Option<String>,
    pub restart_count: u32,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    pub host_port_mappings: HashMap<String, u16>,
    pub network_settings: HashMap<String, String>,
    pub security_context: HashMap<String, serde_json::Value>,
}

impl ContainerState {
    /// Create a new container state
    pub fn new(config_id: uuid::Uuid, image_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            config_id,
            image_id,
            status: ContainerStatus::Created,
            health_status: None,
            restart_count: 0,
            last_updated: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            exit_code: None,
            error_message: None,
            host_port_mappings: HashMap::new(),
            network_settings: HashMap::new(),
            security_context: HashMap::new(),
        }
    }

    /// Check if container is running
    pub fn is_running(&self) -> bool {
        self.status == ContainerStatus::Running
    }

    /// Check if container is stopped
    pub fn is_stopped(&self) -> bool {
        self.status == ContainerStatus::Stopped || self.status == ContainerStatus::Exited
    }

    /// Check if container is healthy
    pub fn is_healthy(&self) -> bool {
        self.health_status.as_deref() == Some("healthy")
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        if let Some(started) = self.started_at {
            let now = chrono::Utc::now();
            now.signed_duration_since(started).num_seconds() as u64
        } else {
            0
        }
    }

    /// Get container age in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = chrono::Utc::now();
        now.signed_duration_since(self.last_updated).num_seconds() as u64
    }

    /// Add port mapping
    pub fn add_port_mapping(&mut self, container_port: String, host_port: u16) {
        self.host_port_mappings.insert(container_port, host_port);
    }

    /// Set network setting
    pub fn set_network_setting(&mut self, key: String, value: String) {
        self.network_settings.insert(key, value);
    }

    /// Set security context value
    pub fn set_security_context(&mut self, key: String, value: serde_json::Value) {
        self.security_context.insert(key, value);
    }

    /// Update status and timestamp
    pub fn update_status(&mut self, status: ContainerStatus) {
        self.status = status.clone();
        self.last_updated = chrono::Utc::now();

        match status {
            ContainerStatus::Running => {
                if self.started_at.is_none() {
                    self.started_at = Some(chrono::Utc::now());
                }
            }
            ContainerStatus::Exited | ContainerStatus::Failed => {
                if self.finished_at.is_none() {
                    self.finished_at = Some(chrono::Utc::now());
                }
            }
            _ => {}
        }
    }

    /// Set health status
    pub fn set_health_status(&mut self, health_status: String) {
        self.health_status = Some(health_status);
        self.last_updated = chrono::Utc::now();
    }

    /// Set exit code and error message
    pub fn set_exit_code(&mut self, exit_code: i32, error_message: Option<String>) {
        self.exit_code = Some(exit_code);
        self.error_message = error_message;
        self.last_updated = chrono::Utc::now();
    }

    /// Increment restart count
    pub fn increment_restart_count(&mut self) {
        self.restart_count += 1;
        self.last_updated = chrono::Utc::now();
    }

    /// Validate container state
    pub fn validate(&self) -> Result<(), String> {
        if self.image_id.is_empty() {
            return Err("Image ID cannot be empty".to_string());
        }

        if self.status == ContainerStatus::Running && self.started_at.is_none() {
            return Err("Running container must have started_at timestamp".to_string());
        }

        if (self.status == ContainerStatus::Exited || self.status == ContainerStatus::Failed)
            && self.finished_at.is_none()
        {
            return Err("Exited/failed container must have finished_at timestamp".to_string());
        }

        Ok(())
    }

    /// Get status as string
    pub fn status_string(&self) -> String {
        match self.status {
            ContainerStatus::Created => "created",
            ContainerStatus::Running => "running",
            ContainerStatus::Stopped => "stopped",
            ContainerStatus::Paused => "paused",
            ContainerStatus::Exited => "exited",
            ContainerStatus::Failed => "failed",
            ContainerStatus::Restarting => "restarting",
            ContainerStatus::Removing => "removing",
            ContainerStatus::Dead => "dead",
        }
        .to_string()
    }

    /// Get container summary
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "status": self.status_string(),
            "image": self.image_id,
            "health": self.health_status,
            "restart_count": self.restart_count,
            "uptime_seconds": self.uptime_seconds(),
            "age_seconds": self.age_seconds(),
            "is_running": self.is_running(),
            "is_stopped": self.is_stopped(),
            "is_healthy": self.is_healthy(),
            "port_mappings": self.host_port_mappings,
            "last_updated": self.last_updated.to_rfc3339(),
        })
    }
}

impl ContainerMetrics {
    /// Create new container metrics
    pub fn new(container_id: uuid::Uuid) -> Self {
        Self {
            container_id,
            timestamp: chrono::Utc::now(),
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0,
            memory_limit_mb: 0,
            memory_percent: 0.0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            block_read_bytes: 0,
            block_write_bytes: 0,
            pids_current: 0,
            pids_limit: 0,
            restart_count: 0,
            uptime_seconds: 0,
            health_status: None,
        }
    }

    /// Calculate memory usage percentage
    pub fn calculate_memory_percent(&mut self) {
        if self.memory_limit_mb > 0 {
            self.memory_percent =
                (self.memory_usage_mb as f64 / self.memory_limit_mb as f64) * 100.0;
        }
    }

    /// Update metrics from Docker stats
    pub fn update_from_stats(&mut self, stats: &serde_json::Value) {
        if let Some(cpu_percent) = stats.get("cpu_percent").and_then(|v| v.as_f64()) {
            self.cpu_usage_percent = cpu_percent;
        }

        if let Some(memory_usage) = stats.get("memory_usage").and_then(|v| v.as_u64()) {
            self.memory_usage_mb = memory_usage / (1024 * 1024);
        }

        if let Some(memory_limit) = stats.get("memory_limit").and_then(|v| v.as_u64()) {
            self.memory_limit_mb = memory_limit / (1024 * 1024);
        }

        if let Some(network_rx) = stats.get("network_rx_bytes").and_then(|v| v.as_u64()) {
            self.network_rx_bytes = network_rx;
        }

        if let Some(network_tx) = stats.get("network_tx_bytes").and_then(|v| v.as_u64()) {
            self.network_tx_bytes = network_tx;
        }

        if let Some(block_read) = stats.get("block_read_bytes").and_then(|v| v.as_u64()) {
            self.block_read_bytes = block_read;
        }

        if let Some(block_write) = stats.get("block_write_bytes").and_then(|v| v.as_u64()) {
            self.block_write_bytes = block_write;
        }

        if let Some(pids) = stats.get("pids_current").and_then(|v| v.as_u64()) {
            self.pids_current = pids as u32;
        }

        if let Some(pids_limit) = stats.get("pids_limit").and_then(|v| v.as_u64()) {
            self.pids_limit = pids_limit as u32;
        }

        if let Some(restarts) = stats.get("restart_count").and_then(|v| v.as_u64()) {
            self.restart_count = restarts as u32;
        }

        if let Some(health) = stats.get("health_status").and_then(|v| v.as_str()) {
            self.health_status = Some(health.to_string());
        }

        self.calculate_memory_percent();
        self.timestamp = chrono::Utc::now();
    }

    /// Get metrics summary
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "container_id": self.container_id,
            "timestamp": self.timestamp.to_rfc3339(),
            "cpu_usage_percent": self.cpu_usage_percent,
            "memory_usage_mb": self.memory_usage_mb,
            "memory_limit_mb": self.memory_limit_mb,
            "memory_percent": self.memory_percent,
            "network_rx_bytes": self.network_rx_bytes,
            "network_tx_bytes": self.network_tx_bytes,
            "block_read_bytes": self.block_read_bytes,
            "block_write_bytes": self.block_write_bytes,
            "pids_current": self.pids_current,
            "pids_limit": self.pids_limit,
            "restart_count": self.restart_count,
            "uptime_seconds": self.uptime_seconds,
            "health_status": self.health_status,
        })
    }
}

impl ContainerEvent {
    /// Create new container event
    pub fn new(container_id: uuid::Uuid, event_type: String, message: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            container_id,
            event_type,
            message,
            timestamp: chrono::Utc::now(),
            level: "info".to_string(),
            details: HashMap::new(),
        }
    }

    /// Create error event
    pub fn error(container_id: uuid::Uuid, event_type: String, message: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            container_id,
            event_type,
            message,
            timestamp: chrono::Utc::now(),
            level: "error".to_string(),
            details: HashMap::new(),
        }
    }

    /// Create warning event
    pub fn warning(container_id: uuid::Uuid, event_type: String, message: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            container_id,
            event_type,
            message,
            timestamp: chrono::Utc::now(),
            level: "warning".to_string(),
            details: HashMap::new(),
        }
    }

    /// Add detail to event
    pub fn add_detail(&mut self, key: String, value: String) {
        self.details.insert(key, value);
    }

    /// Set event level
    pub fn set_level(&mut self, level: String) {
        self.level = level;
    }

    /// Validate event
    pub fn validate(&self) -> Result<(), String> {
        if self.event_type.is_empty() {
            return Err("Event type cannot be empty".to_string());
        }

        if self.message.is_empty() {
            return Err("Event message cannot be empty".to_string());
        }

        Ok(())
    }

    /// Get event summary
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "container_id": self.container_id,
            "event_type": self.event_type,
            "message": self.message,
            "timestamp": self.timestamp.to_rfc3339(),
            "level": self.level,
            "details": self.details,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_state_creation() {
        let config_id = uuid::Uuid::new_v4();
        let image_id = "test:latest".to_string();
        let state = ContainerState::new(config_id, image_id.clone());

        assert_eq!(state.config_id, config_id);
        assert_eq!(state.image_id, image_id);
        assert_eq!(state.status, ContainerStatus::Created);
        assert_eq!(state.restart_count, 0);
        assert!(!state.is_running());
        assert!(state.is_stopped());
        assert!(!state.is_healthy());
    }

    #[test]
    fn test_container_state_status_updates() {
        let config_id = uuid::Uuid::new_v4();
        let mut state = ContainerState::new(config_id, "test:latest".to_string());

        state.update_status(ContainerStatus::Running);
        assert!(state.is_running());
        assert!(state.started_at.is_some());

        state.update_status(ContainerStatus::Exited);
        assert!(state.is_stopped());
        assert!(state.finished_at.is_some());
    }

    #[test]
    fn test_container_state_health() {
        let config_id = uuid::Uuid::new_v4();
        let mut state = ContainerState::new(config_id, "test:latest".to_string());

        assert!(!state.is_healthy());

        state.set_health_status("healthy".to_string());
        assert!(state.is_healthy());

        state.set_health_status("unhealthy".to_string());
        assert!(!state.is_healthy());
    }

    #[test]
    fn test_container_metrics_creation() {
        let container_id = uuid::Uuid::new_v4();
        let metrics = ContainerMetrics::new(container_id);

        assert_eq!(metrics.container_id, container_id);
        assert_eq!(metrics.cpu_usage_percent, 0.0);
        assert_eq!(metrics.memory_usage_mb, 0);
        assert_eq!(metrics.memory_percent, 0.0);
    }

    #[test]
    fn test_container_metrics_memory_calculation() {
        let container_id = uuid::Uuid::new_v4();
        let mut metrics = ContainerMetrics::new(container_id);

        metrics.memory_usage_mb = 512;
        metrics.memory_limit_mb = 1024;
        metrics.calculate_memory_percent();

        assert_eq!(metrics.memory_percent, 50.0);
    }

    #[test]
    fn test_container_event_creation() {
        let container_id = uuid::Uuid::new_v4();
        let event = ContainerEvent::new(
            container_id,
            "test_event".to_string(),
            "Test message".to_string(),
        );

        assert_eq!(event.container_id, container_id);
        assert_eq!(event.event_type, "test_event");
        assert_eq!(event.message, "Test message");
        assert_eq!(event.level, "info");
    }

    #[test]
    fn test_container_event_levels() {
        let container_id = uuid::Uuid::new_v4();

        let info_event = ContainerEvent::new(
            container_id,
            "info_event".to_string(),
            "Info message".to_string(),
        );
        assert_eq!(info_event.level, "info");

        let error_event = ContainerEvent::error(
            container_id,
            "error_event".to_string(),
            "Error message".to_string(),
        );
        assert_eq!(error_event.level, "error");

        let warning_event = ContainerEvent::warning(
            container_id,
            "warning_event".to_string(),
            "Warning message".to_string(),
        );
        assert_eq!(warning_event.level, "warning");
    }

    #[test]
    fn test_container_state_validation() {
        let config_id = uuid::Uuid::new_v4();
        let mut state = ContainerState::new(config_id, "test:latest".to_string());

        // Valid state
        assert!(state.validate().is_ok());

        // Invalid - empty image ID
        state.image_id = "".to_string();
        assert!(state.validate().is_err());

        // Invalid - running without started_at
        state.image_id = "test:latest".to_string();
        state.status = ContainerStatus::Running;
        state.started_at = None;
        assert!(state.validate().is_err());

        // Invalid - exited without finished_at
        state.status = ContainerStatus::Exited;
        state.finished_at = None;
        assert!(state.validate().is_err());
    }
}
