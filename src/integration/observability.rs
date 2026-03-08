//! Observability service for Merlin AI Router Docker deployment
//! Provides metrics collection, structured logging, and monitoring

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error, debug, span, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use prometheus::{Counter, Histogram, Gauge, Registry, TextEncoder, Encoder};
use crate::models::container_state::{ContainerState, ContainerStatus};
use crate::integration::docker_client::DockerConfigError;

/// Observability metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityMetrics {
    pub timestamp: DateTime<Utc>,
    pub component: String,
    pub metrics: HashMap<String, MetricValue>,
    pub tags: HashMap<String, String>,
}

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(f64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Boolean(bool),
    String(String),
}

/// Log entry with structured data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub component: String,
    pub message: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub trace_id: Option<Uuid>,
}

/// Log levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub enable_metrics: bool,
    pub enable_logging: bool,
    pub enable_tracing: bool,
    pub metrics_retention_hours: u64,
    pub log_retention_hours: u64,
    pub prometheus_port: u16,
    pub log_level: String,
    pub export_interval_seconds: u64,
}

/// Observability service
#[derive(Clone)]
pub struct ObservabilityService {
    metrics: Arc<RwLock<Vec<ObservabilityMetrics>>>,
    logs: Arc<RwLock<Vec<LogEntry>>>,
    registry: Registry,
    counters: Arc<RwLock<HashMap<String, Counter>>>,
    gauges: Arc<RwLock<HashMap<String, Gauge>>>,
    histograms: Arc<RwLock<HashMap<String, Histogram>>>,
    config: ObservabilityConfig,
}

/// Predefined metrics
pub struct Metrics {
    pub containers_created: Counter,
    pub containers_started: Counter,
    pub containers_stopped: Counter,
    pub containers_failed: Counter,
    pub security_scans_performed: Counter,
    pub security_vulnerabilities_found: Counter,
    pub deployment_success: Counter,
    pub deployment_failures: Counter,
    pub resource_memory_usage: Gauge,
    pub resource_cpu_usage: Gauge,
    pub request_duration: Histogram,
}

impl ObservabilityService {
    pub fn new(config: ObservabilityConfig) -> Result<Self, DockerConfigError> {
        let registry = Registry::new();

        // Initialize metrics
        let counters = Arc::new(RwLock::new(HashMap::new()));
        let gauges = Arc::new(RwLock::new(HashMap::new()));
        let histograms = Arc::new(RwLock::new(HashMap::new()));

        let service = Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            logs: Arc::new(RwLock::new(Vec::new())),
            registry,
            counters,
            gauges,
            histograms,
            config,
        };

        // Initialize predefined metrics
        service.initialize_metrics()?;

        // Setup logging if enabled
        if config.enable_logging {
            service.setup_logging()?;
        }

        Ok(service)
    }

    /// Initialize predefined metrics
    fn initialize_metrics(&self) -> Result<(), DockerConfigError> {
        let mut counters = self.counters.write().unwrap();
        let mut gauges = self.gauges.write().unwrap();
        let mut histograms = self.histograms.write().unwrap();

        // Container metrics
        counters.insert(
            "containers_created".to_string(),
            Counter::new("containers_created_total", "Total containers created")?
        );
        counters.insert(
            "containers_started".to_string(),
            Counter::new("containers_started_total", "Total containers started")?
        );
        counters.insert(
            "containers_stopped".to_string(),
            Counter::new("containers_stopped_total", "Total containers stopped")?
        );
        counters.insert(
            "containers_failed".to_string(),
            Counter::new("containers_failed_total", "Total containers failed")?
        );

        // Security metrics
        counters.insert(
            "security_scans_performed".to_string(),
            Counter::new("security_scans_performed_total", "Total security scans performed")?
        );
        counters.insert(
            "security_vulnerabilities_found".to_string(),
            Counter::new("security_vulnerabilities_found_total", "Total security vulnerabilities found")?
        );

        // Deployment metrics
        counters.insert(
            "deployment_success".to_string(),
            Counter::new("deployment_success_total", "Total successful deployments")?
        );
        counters.insert(
            "deployment_failures".to_string(),
            Counter::new("deployment_failures_total", "Total deployment failures")?
        );

        // Resource metrics
        gauges.insert(
            "resource_memory_usage_bytes".to_string(),
            Gauge::new("resource_memory_usage_bytes", "Current memory usage in bytes")?
        );
        gauges.insert(
            "resource_cpu_usage_percent".to_string(),
            Gauge::new("resource_cpu_usage_percent", "Current CPU usage percentage")?
        );

        // Request metrics
        histograms.insert(
            "request_duration_seconds".to_string(),
            Histogram::new("request_duration_seconds", "Request duration in seconds")?
        );

        // Register all metrics
        for counter in counters.values() {
            self.registry.register(Box::new(counter.clone()))?;
        }

        for gauge in gauges.values() {
            self.registry.register(Box::new(gauge.clone()))?;
        }

        for histogram in histograms.values() {
            self.registry.register(Box::new(histogram.clone()))?;
        }

        Ok(())
    }

    /// Setup structured logging
    fn setup_logging(&self) -> Result<(), DockerConfigError> {
        let level = match self.config.log_level.as_str() {
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        };

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_PKG_NAME")).into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        info!("Observability service initialized with logging enabled");
        Ok(())
    }

    /// Record a metric
    pub async fn record_metric(
        &self,
        component: &str,
        metric_name: &str,
        value: MetricValue,
        tags: Option<HashMap<String, String>>,
    ) -> Result<(), DockerConfigError> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        let metric = ObservabilityMetrics {
            timestamp: Utc::now(),
            component: component.to_string(),
            metrics: {
                let mut metrics = HashMap::new();
                metrics.insert(metric_name.to_string(), value);
                metrics
            },
            tags: tags.unwrap_or_default(),
        };

        let mut metrics_store = self.metrics.write().await;
        metrics_store.push(metric);

        // Cleanup old metrics
        let cutoff = Utc::now() - chrono::Duration::hours(self.config.metrics_retention_hours as i64);
        metrics_store.retain(|m| m.timestamp > cutoff);

        Ok(())
    }

    /// Increment a counter metric
    pub async fn increment_counter(
        &self,
        metric_name: &str,
        value: f64,
        tags: Option<HashMap<String, String>>,
    ) -> Result<(), DockerConfigError> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        let counters = self.counters.read().unwrap();
        if let Some(counter) = counters.get(metric_name) {
            counter.inc_by(value);
        }

        Ok(())
    }

    /// Set a gauge metric
    pub async fn set_gauge(
        &self,
        metric_name: &str,
        value: f64,
        tags: Option<HashMap<String, String>>,
    ) -> Result<(), DockerConfigError> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        let gauges = self.gauges.read().unwrap();
        if let Some(gauge) = gauges.get(metric_name) {
            gauge.set(value);
        }

        Ok(())
    }

    /// Record a histogram observation
    pub async fn observe_histogram(
        &self,
        metric_name: &str,
        value: f64,
        tags: Option<HashMap<String, String>>,
    ) -> Result<(), DockerConfigError> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        let histograms = self.histograms.read().unwrap();
        if let Some(histogram) = histograms.get(metric_name) {
            histogram.observe(value);
        }

        Ok(())
    }

    /// Log a structured entry
    pub async fn log(
        &self,
        level: LogLevel,
        component: &str,
        message: &str,
        fields: Option<HashMap<String, serde_json::Value>>,
        trace_id: Option<Uuid>,
    ) -> Result<(), DockerConfigError> {
        if !self.config.enable_logging {
            return Ok(());
        }

        let entry = LogEntry {
            timestamp: Utc::now(),
            level,
            component: component.to_string(),
            message: message.to_string(),
            fields: fields.unwrap_or_default(),
            trace_id,
        };

        // Store log entry
        let mut logs = self.logs.write().await;
        logs.push(entry.clone());

        // Cleanup old logs
        let cutoff = Utc::now() - chrono::Duration::hours(self.config.log_retention_hours as i64);
        logs.retain(|log| log.timestamp > cutoff);

        // Also emit to tracing
        match level {
            LogLevel::Debug => debug!(target: component, "{}", message),
            LogLevel::Info => info!(target: component, "{}", message),
            LogLevel::Warning => warn!(target: component, "{}", message),
            LogLevel::Error => error!(target: component, "{}", message),
            LogLevel::Critical => error!(target: component, "CRITICAL: {}", message),
        }

        Ok(())
    }

    /// Get metrics in Prometheus format
    pub async fn get_prometheus_metrics(&self) -> Result<String, DockerConfigError> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let result = encoder.encode_to_string(&metric_families)?;
        Ok(result)
    }

    /// Get all metrics
    pub async fn get_metrics(&self, limit: Option<usize>) -> Vec<ObservabilityMetrics> {
        let metrics = self.metrics.read().await;
        match limit {
            Some(limit) => metrics[metrics.len().saturating_sub(limit)..].to_vec(),
            None => metrics.clone(),
        }
    }

    /// Get all logs
    pub async fn get_logs(&self, limit: Option<usize>) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        match limit {
            Some(limit) => logs[logs.len().saturating_sub(limit)..].to_vec(),
            None => logs.clone(),
        }
    }

    /// Get logs by component
    pub async fn get_logs_by_component(&self, component: &str, limit: Option<usize>) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        let filtered_logs: Vec<LogEntry> = logs
            .iter()
            .filter(|log| log.component == component)
            .cloned()
            .collect();

        match limit {
            Some(limit) => filtered_logs[filtered_logs.len().saturating_sub(limit)..].to_vec(),
            None => filtered_logs,
        }
    }

    /// Query metrics with filters
    pub async fn query_metrics(
        &self,
        component_filter: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Vec<ObservabilityMetrics> {
        let metrics = self.metrics.read().await;

        metrics
            .iter()
            .filter(|metric| {
                if let Some(component) = component_filter {
                    if metric.component != component {
                        return false;
                    }
                }

                if let Some(start) = start_time {
                    if metric.timestamp < start {
                        return false;
                    }
                }

                if let Some(end) = end_time {
                    if metric.timestamp > end {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    /// Get metrics summary statistics
    pub async fn get_metrics_summary(&self) -> HashMap<String, serde_json::Value> {
        let metrics = self.metrics.read().await;
        let logs = self.logs.read().await;

        let mut summary = HashMap::new();

        // Metrics count
        summary.insert(
            "total_metrics".to_string(),
            serde_json::Value::Number(serde_json::Number::from(metrics.len())),
        );

        // Logs count
        summary.insert(
            "total_logs".to_string(),
            serde_json::Value::Number(serde_json::Number::from(logs.len())),
        );

        // Unique components
        let unique_components: std::collections::HashSet<String> = metrics
            .iter()
            .map(|m| m.component.clone())
            .collect();
        summary.insert(
            "unique_components".to_string(),
            serde_json::Value::Number(serde_json::Number::from(unique_components.len())),
        );

        // Log levels distribution
        let mut log_levels = HashMap::new();
        for log in logs.iter() {
            let level = format!("{:?}", log.level);
            *log_levels.entry(level).or_insert(0) += 1;
        }
        summary.insert(
            "log_levels".to_string(),
            serde_json::Value::Object(log_levels.into_iter().map(|(k, v)| (k, serde_json::Value::Number(serde_json::Number::from(v)))).collect()),
        );

        summary
    }

    /// Start metrics export loop
    pub async fn start_export_loop(&self) -> Result<(), DockerConfigError> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        let service = self.clone();
        let interval = self.config.export_interval_seconds;

        tokio::spawn(async move {
            loop {
                // Export metrics to Prometheus format
                if let Ok(metrics) = service.get_prometheus_metrics().await {
                    debug!("Exported {} metrics", metrics.lines().count());
                }

                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            }
        });

        Ok(())
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            enable_logging: true,
            enable_tracing: true,
            metrics_retention_hours: 24,
            log_retention_hours: 168, // 7 days
            prometheus_port: 9090,
            log_level: "info".to_string(),
            export_interval_seconds: 30,
        }
    }
}

/// Convenience macro for logging
#[macro_export]
macro_rules! obs_log {
    ($service:expr, $level:expr, $component:expr, $message:expr) => {
        $service.log($level, $component, $message, None, None).await
    };
    ($service:expr, $level:expr, $component:expr, $message:expr, $($field:tt)*) => {
        {
            let mut fields = std::collections::HashMap::new();
            $(
                obs_log_fields!(&mut fields, $($field)*);
            )*
            $service.log($level, $component, $message, Some(fields), None).await
        }
    };
}

/// Helper macro for field processing
#[macro_export]
macro_rules! obs_log_fields {
    ($fields:expr, $key:literal = $value:expr) => {
        $fields.insert($key.to_string(), serde_json::Value::from($value));
    };
}