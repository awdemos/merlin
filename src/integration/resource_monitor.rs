//! Resource monitoring service for Merlin AI Router
//!
//! Provides real-time monitoring of container resources including CPU, memory,
//! disk I/O, network I/O, and custom metrics with alerting capabilities.

use crate::models::container_state::{ContainerMetrics, ContainerState, ContainerStatus};
use crate::models::docker_config::DockerConfigError;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use tracing::{info, warn, error, debug};

/// Resource monitoring service
pub struct ResourceMonitor {
    /// Metrics storage for all containers
    metrics: Arc<RwLock<HashMap<uuid::Uuid, Vec<ContainerMetrics>>>>,

    /// Alert thresholds
    alert_thresholds: Arc<RwLock<AlertThresholds>>,

    /// Monitoring configuration
    config: MonitoringConfig,

    /// Metrics history (rolling window)
    metrics_history: Arc<RwLock<HashMap<uuid::Uuid, VecDeque<ContainerMetrics>>>>,

    /// Alert history
    alert_history: Arc<RwLock<Vec<Alert>>>,
}

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Collection interval in seconds
    pub collection_interval: u64,

    /// Metrics retention period in hours
    pub retention_hours: u64,

    /// Maximum history size per container
    pub max_history_size: usize,

    /// Enable detailed metrics
    pub detailed_metrics: bool,

    /// Enable alerting
    pub enable_alerting: bool,

    /// Alert cooldown period in seconds
    pub alert_cooldown_seconds: u64,
}

/// Alert thresholds configuration
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// CPU usage threshold percentage
    pub cpu_threshold_percent: f64,

    /// Memory usage threshold percentage
    pub memory_threshold_percent: f64,

    /// Disk I/O threshold MB/s
    pub disk_io_threshold_mb_s: f64,

    /// Network I/O threshold MB/s
    pub network_io_threshold_mb_s: f64,

    /// PIDS count threshold
    pub pids_threshold: u32,

    /// Container restart threshold
    pub restart_threshold: u32,

    /// Uptime threshold in seconds
    pub uptime_threshold_seconds: u64,
}

/// Alert information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Alert {
    pub id: uuid::Uuid,
    pub container_id: uuid::Uuid,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub value: serde_json::Value,
    pub threshold: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub resolved: bool,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: HashMap<String, String>,
}

/// Alert types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum AlertType {
    HighCpuUsage,
    HighMemoryUsage,
    HighDiskIo,
    HighNetworkIo,
    HighPidCount,
    ContainerRestarted,
    ContainerUnhealthy,
    ContainerDown,
    ResourceLimitExceeded,
    Custom(String),
}

/// Alert severity levels
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Resource usage summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceSummary {
    pub total_containers: usize,
    pub running_containers: usize,
    pub total_cpu_percent: f64,
    pub total_memory_mb: u64,
    pub total_memory_percent: f64,
    pub total_disk_io_mb_s: f64,
    pub total_network_io_mb_s: f64,
    pub total_pids: u32,
    pub active_alerts: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Resource trend analysis
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceTrend {
    pub container_id: uuid::Uuid,
    pub metric_type: String,
    pub trend_direction: TrendDirection,
    pub change_percent: f64,
    pub time_window_minutes: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Trend direction
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            collection_interval: 30,
            retention_hours: 24,
            max_history_size: 1000,
            detailed_metrics: true,
            enable_alerting: true,
            alert_cooldown_seconds: 300,
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_threshold_percent: 80.0,
            memory_threshold_percent: 85.0,
            disk_io_threshold_mb_s: 100.0,
            network_io_threshold_mb_s: 100.0,
            pids_threshold: 1000,
            restart_threshold: 5,
            uptime_threshold_seconds: 3600,
        }
    }
}

impl ResourceMonitor {
    /// Create new resource monitor
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            alert_thresholds: Arc::new(RwLock::new(AlertThresholds::default())),
            config,
            metrics_history: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create resource monitor with custom thresholds
    pub fn with_thresholds(config: MonitoringConfig, thresholds: AlertThresholds) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            alert_thresholds: Arc::new(RwLock::new(thresholds)),
            config,
            metrics_history: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start monitoring loop
    pub async fn start_monitoring(&self, container_ids: Vec<uuid::Uuid>) -> Result<(), DockerConfigError> {
        info!("Starting resource monitoring for {} containers", container_ids.len());

        // Initialize metrics storage for containers
        let mut metrics = self.metrics.write().await;
        let mut history = self.metrics_history.write().await;

        for container_id in container_ids {
            metrics.insert(container_id, Vec::new());
            history.insert(container_id, VecDeque::new());
        }
        drop(metrics);
        drop(history);

        // Start monitoring loop
        let monitor_self = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = monitor_self.collect_metrics().await {
                    error!("Failed to collect metrics: {}", e);
                }

                if let Err(e) = monitor_self.check_alerts().await {
                    error!("Failed to check alerts: {}", e);
                }

                if let Err(e) = monitor_self.cleanup_old_metrics().await {
                    error!("Failed to cleanup old metrics: {}", e);
                }

                tokio::time::sleep(Duration::from_secs(monitor_self.config.collection_interval)).await;
            }
        });

        Ok(())
    }

    /// Add container to monitoring
    pub async fn add_container(&self, container_id: uuid::Uuid) -> Result<(), DockerConfigError> {
        let mut metrics = self.metrics.write().await;
        let mut history = self.metrics_history.write().await;

        metrics.insert(container_id, Vec::new());
        history.insert(container_id, VecDeque::with_capacity(self.config.max_history_size));

        info!("Added container {} to monitoring", container_id);
        Ok(())
    }

    /// Remove container from monitoring
    pub async fn remove_container(&self, container_id: uuid::Uuid) -> Result<(), DockerConfigError> {
        let mut metrics = self.metrics.write().await;
        let mut history = self.metrics_history.write().await;

        metrics.remove(&container_id);
        history.remove(&container_id);

        info!("Removed container {} from monitoring", container_id);
        Ok(())
    }

    /// Collect metrics for all monitored containers
    async fn collect_metrics(&self) -> Result<(), DockerConfigError> {
        let container_ids: Vec<uuid::Uuid> = {
            let metrics = self.metrics.read().await;
            metrics.keys().cloned().collect()
        };

        for container_id in container_ids {
            if let Err(e) = self.collect_container_metrics(container_id).await {
                warn!("Failed to collect metrics for container {}: {}", container_id, e);
            }
        }

        Ok(())
    }

    /// Collect metrics for a specific container
    async fn collect_container_metrics(&self, container_id: uuid::Uuid) -> Result<(), DockerConfigError> {
        // In a real implementation, this would collect actual Docker stats
        // For now, we'll simulate metrics collection
        let metrics = ContainerMetrics {
            container_id,
            timestamp: chrono::Utc::now(),
            cpu_usage_percent: rand::random::<f64>() * 100.0,
            memory_usage_mb: (rand::random::<f64>() * 2048.0) as u64,
            memory_limit_mb: 4096,
            memory_percent: 0.0,
            network_rx_bytes: (rand::random::<f64>() * 1_000_000.0) as u64,
            network_tx_bytes: (rand::random::<f64>() * 1_000_000.0) as u64,
            block_read_bytes: (rand::random::<f64>() * 100_000.0) as u64,
            block_write_bytes: (rand::random::<f64>() * 100_000.0) as u64,
            pids_current: (rand::random::<f32>() * 500.0) as u32,
            pids_limit: 1000,
            restart_count: 0,
            uptime_seconds: (rand::random::<f64>() * 86400.0) as u64,
            health_status: Some("healthy".to_string()),
        };

        // Store metrics
        {
            let mut container_metrics = self.metrics.write().await;
            if let Some(metrics_list) = container_metrics.get_mut(&container_id) {
                metrics_list.push(metrics.clone());
            }
        }

        // Store in history
        {
            let mut history = self.metrics_history.write().await;
            if let Some(container_history) = history.get_mut(&container_id) {
                if container_history.len() >= self.config.max_history_size {
                    container_history.pop_front();
                }
                container_history.push_back(metrics);
            }
        }

        Ok(())
    }

    /// Check for alert conditions
    async fn check_alerts(&self) -> Result<(), DockerConfigError> {
        let thresholds = self.alert_thresholds.read().await;
        let container_ids: Vec<uuid::Uuid> = {
            let metrics = self.metrics.read().await;
            metrics.keys().cloned().collect()
        };

        for container_id in container_ids {
            let latest_metrics = {
                let metrics = self.metrics.read().await;
                metrics.get(&container_id)
                    .and_then(|m| m.last().cloned())
            };

            if let Some(metrics) = latest_metrics {
                // Check CPU threshold
                if metrics.cpu_usage_percent > thresholds.cpu_threshold_percent {
                    self.create_alert(
                        container_id,
                        AlertType::HighCpuUsage,
                        AlertSeverity::High,
                        format!("CPU usage is {:.1}%", metrics.cpu_usage_percent),
                        json!(metrics.cpu_usage_percent),
                        json!(thresholds.cpu_threshold_percent),
                    ).await?;
                }

                // Check memory threshold
                if metrics.memory_percent > thresholds.memory_threshold_percent {
                    self.create_alert(
                        container_id,
                        AlertType::HighMemoryUsage,
                        AlertSeverity::High,
                        format!("Memory usage is {:.1}%", metrics.memory_percent),
                        json!(metrics.memory_percent),
                        json!(thresholds.memory_threshold_percent),
                    ).await?;
                }

                // Check PIDS threshold
                if metrics.pids_current > thresholds.pids_threshold {
                    self.create_alert(
                        container_id,
                        AlertType::HighPidCount,
                        AlertSeverity::Medium,
                        format!("PID count is {}", metrics.pids_current),
                        json!(metrics.pids_current),
                        json!(thresholds.pids_threshold),
                    ).await?;
                }
            }
        }

        Ok(())
    }

    /// Create and store alert
    async fn create_alert(
        &self,
        container_id: uuid::Uuid,
        alert_type: AlertType,
        severity: AlertSeverity,
        message: String,
        value: serde_json::Value,
        threshold: serde_json::Value,
    ) -> Result<(), DockerConfigError> {
        // Check cooldown period
        let last_alert_time = {
            let alerts = self.alert_history.read().await;
            alerts.iter()
                .filter(|a| a.container_id == container_id && a.alert_type == alert_type)
                .filter_map(|a| Some(a.timestamp))
                .max()
        };

        if let Some(last_time) = last_alert_time {
            let cooldown = chrono::Duration::seconds(self.config.alert_cooldown_seconds as i64);
            if chrono::Utc::now().signed_duration_since(last_time) < cooldown {
                return Ok(()); // Skip alert due to cooldown
            }
        }

        let alert = Alert {
            id: uuid::Uuid::new_v4(),
            container_id,
            alert_type,
            severity,
            message,
            value,
            threshold,
            timestamp: chrono::Utc::now(),
            resolved: false,
            resolved_at: None,
            metadata: HashMap::new(),
        };

        let mut alerts = self.alert_history.write().await;
        alerts.push(alert.clone());

        // Limit alert history size
        if alerts.len() > 1000 {
            alerts.remove(0);
        }

        warn!("Alert triggered for container {}: {}", container_id, alert.message);

        // Here you would integrate with alerting systems like Slack, PagerDuty, etc.
        self.send_alert_notification(&alert).await?;

        Ok(())
    }

    /// Send alert notification (placeholder implementation)
    async fn send_alert_notification(&self, alert: &Alert) -> Result<(), DockerConfigError> {
        debug!("Sending alert notification: {:?}", alert);
        // In a real implementation, this would send to Slack, PagerDuty, email, etc.
        Ok(())
    }

    /// Cleanup old metrics based on retention policy
    async fn cleanup_old_metrics(&self) -> Result<(), DockerConfigError> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(self.config.retention_hours as i64);

        let mut metrics = self.metrics.write().await;
        for (container_id, container_metrics) in metrics.iter_mut() {
            container_metrics.retain(|m| m.timestamp > cutoff_time);
        }

        let mut history = self.metrics_history.write().await;
        for (container_id, container_history) in history.iter_mut() {
            container_history.retain(|m| m.timestamp > cutoff_time);
        }

        Ok(())
    }

    /// Get metrics for a specific container
    pub async fn get_container_metrics(&self, container_id: uuid::Uuid) -> Result<Vec<ContainerMetrics>, DockerConfigError> {
        let metrics = self.metrics.read().await;
        Ok(metrics.get(&container_id).cloned().unwrap_or_else(Vec::new))
    }

    /// Get resource summary
    pub async fn get_resource_summary(&self) -> Result<ResourceSummary, DockerConfigError> {
        let metrics = self.metrics.read().await;
        let alerts = self.alert_history.read().await;

        let mut total_cpu = 0.0;
        let mut total_memory = 0;
        let mut total_memory_percent = 0.0;
        let mut total_containers = 0;
        let mut active_alerts = 0;

        for (container_id, container_metrics) in metrics.iter() {
            if let Some(latest) = container_metrics.last() {
                total_cpu += latest.cpu_usage_percent;
                total_memory += latest.memory_usage_mb;
                total_memory_percent += latest.memory_percent;
                total_containers += 1;
            }
        }

        active_alerts = alerts.iter().filter(|a| !a.resolved).count();

        Ok(ResourceSummary {
            total_containers,
            running_containers: total_containers, // Simplified
            total_cpu_percent: total_cpu,
            total_memory_mb: total_memory,
            total_memory_percent,
            total_disk_io_mb_s: 0.0, // Would calculate from metrics
            total_network_io_mb_s: 0.0, // Would calculate from metrics
            total_pids: 0, // Would calculate from metrics
            active_alerts,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>, DockerConfigError> {
        let alerts = self.alert_history.read().await;
        Ok(alerts.iter().filter(|a| !a.resolved).cloned().collect())
    }

    /// Get alert history
    pub async fn get_alert_history(&self, limit: Option<usize>) -> Result<Vec<Alert>, DockerConfigError> {
        let alerts = self.alert_history.read().await;
        let mut history = alerts.clone();
        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(lim) = limit {
            history.truncate(lim);
        }

        Ok(history)
    }

    /// Resolve alert
    pub async fn resolve_alert(&self, alert_id: uuid::Uuid) -> Result<bool, DockerConfigError> {
        let mut alerts = self.alert_history.write().await;

        for alert in alerts.iter_mut() {
            if alert.id == alert_id {
                alert.resolved = true;
                alert.resolved_at = Some(chrono::Utc::now());
                info!("Resolved alert: {}", alert.message);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Analyze resource trends
    pub async fn analyze_trends(&self, container_id: uuid::Uuid, window_minutes: u32) -> Result<Vec<ResourceTrend>, DockerConfigError> {
        let history = self.metrics_history.read().await;
        let container_history = history.get(&container_id)
            .map(|h| h.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_else(Vec::new);

        let cutoff_time = chrono::Utc::now() - chrono::Duration::minutes(window_minutes as i64);
        let recent_metrics: Vec<_> = container_history
            .into_iter()
            .filter(|m| m.timestamp > cutoff_time)
            .collect();

        let mut trends = Vec::new();

        if recent_metrics.len() >= 2 {
            // CPU trend
            let cpu_trend = self.calculate_trend(&recent_metrics, "cpu_usage_percent");
            trends.push(cpu_trend);

            // Memory trend
            let memory_trend = self.calculate_trend(&recent_metrics, "memory_usage_percent");
            trends.push(memory_trend);

            // Network I/O trend
            let network_trend = self.calculate_trend(&recent_metrics, "network_io_mb_s");
            trends.push(network_trend);
        }

        Ok(trends)
    }

    /// Calculate trend for a specific metric
    fn calculate_trend(&self, metrics: &[ContainerMetrics], metric_type: &str) -> ResourceTrend {
        if metrics.len() < 2 {
            return ResourceTrend {
                container_id: metrics[0].container_id,
                metric_type: metric_type.to_string(),
                trend_direction: TrendDirection::Stable,
                change_percent: 0.0,
                time_window_minutes: 0,
                timestamp: chrono::Utc::now(),
            };
        }

        let first = metrics.first().unwrap();
        let last = metrics.last().unwrap();

        let (first_value, last_value) = match metric_type {
            "cpu_usage_percent" => (first.cpu_usage_percent, last.cpu_usage_percent),
            "memory_usage_percent" => (first.memory_percent, last.memory_percent),
            "network_io_mb_s" => {
                let first_net = (first.network_rx_bytes + first.network_tx_bytes) as f64 / (1024.0 * 1024.0);
                let last_net = (last.network_rx_bytes + last.network_tx_bytes) as f64 / (1024.0 * 1024.0);
                (first_net, last_net)
            }
            _ => (0.0, 0.0),
        };

        let change_percent = if first_value > 0.0 {
            ((last_value - first_value) / first_value) * 100.0
        } else {
            0.0
        };

        let trend_direction = if change_percent > 5.0 {
            TrendDirection::Increasing
        } else if change_percent < -5.0 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        let time_window = ((last.timestamp - first.timestamp).num_minutes().max(1)) as u32;

        ResourceTrend {
            container_id: first.container_id,
            metric_type: metric_type.to_string(),
            trend_direction,
            change_percent,
            time_window_minutes: time_window,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl Clone for ResourceMonitor {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
            alert_thresholds: Arc::clone(&self.alert_thresholds),
            config: self.config.clone(),
            metrics_history: Arc::clone(&self.metrics_history),
            alert_history: Arc::clone(&self.alert_history),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let config = MonitoringConfig::default();
        let monitor = ResourceMonitor::new(config);
        assert_eq!(monitor.config.collection_interval, 30);
    }

    #[test]
    fn test_alert_thresholds_default() {
        let thresholds = AlertThresholds::default();
        assert_eq!(thresholds.cpu_threshold_percent, 80.0);
        assert_eq!(thresholds.memory_threshold_percent, 85.0);
    }

    #[tokio::test]
    async fn test_add_remove_container() {
        let config = MonitoringConfig::default();
        let monitor = ResourceMonitor::new(config);
        let container_id = uuid::Uuid::new_v4();

        assert!(monitor.add_container(container_id).await.is_ok());
        assert!(monitor.remove_container(container_id).await.is_ok());
    }

    #[tokio::test]
    async fn test_resource_summary() {
        let config = MonitoringConfig::default();
        let monitor = ResourceMonitor::new(config);

        let summary = monitor.get_resource_summary().await.unwrap();
        assert_eq!(summary.total_containers, 0);
        assert_eq!(summary.active_alerts, 0);
    }

    #[test]
    fn test_trend_calculation() {
        let config = MonitoringConfig::default();
        let monitor = ResourceMonitor::new(config);
        let container_id = uuid::Uuid::new_v4();

        let metrics = vec![
            ContainerMetrics::new(container_id),
            ContainerMetrics::new(container_id),
        ];

        let trend = monitor.calculate_trend(&metrics, "cpu_usage_percent");
        assert_eq!(trend.metric_type, "cpu_usage_percent");
        assert_eq!(trend.container_id, container_id);
    }
}