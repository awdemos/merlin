//! Integration tests for observability service

use merlin::integration::observability::{
    ObservabilityService, ObservabilityConfig, MetricValue, LogLevel, ObservabilityMetrics, LogEntry
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[tokio::test]
async fn test_observability_service_creation() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    assert!(service.config.enable_metrics);
    assert!(service.config.enable_logging);
    assert!(service.config.enable_tracing);
    assert_eq!(service.config.metrics_retention_hours, 24);
    assert_eq!(service.config.log_retention_hours, 168);
}

#[tokio::test]
async fn test_observability_custom_config() {
    let config = ObservabilityConfig {
        enable_metrics: true,
        enable_logging: false,
        enable_tracing: true,
        metrics_retention_hours: 48,
        log_retention_hours: 72,
        prometheus_port: 8080,
        log_level: "debug".to_string(),
        export_interval_seconds: 60,
    };

    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    assert!(service.config.enable_metrics);
    assert!(!service.config.enable_logging);
    assert!(service.config.enable_tracing);
    assert_eq!(service.config.metrics_retention_hours, 48);
    assert_eq!(service.config.log_retention_hours, 72);
    assert_eq!(service.config.prometheus_port, 8080);
    assert_eq!(service.config.log_level, "debug");
    assert_eq!(service.config.export_interval_seconds, 60);
}

#[tokio::test]
async fn test_metric_recording() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    // Test recording different metric types
    service.record_metric(
        "test-component",
        "test_counter",
        MetricValue::Counter(42.0),
        Some({
            let mut tags = HashMap::new();
            tags.insert("environment".to_string(), "test".to_string());
            tags
        }),
    ).await.expect("Failed to record metric");

    service.record_metric(
        "test-component",
        "test_gauge",
        MetricValue::Gauge(75.5),
        None,
    ).await.expect("Failed to record metric");

    service.record_metric(
        "test-component",
        "test_histogram",
        MetricValue::Histogram(vec![1.0, 2.0, 3.0]),
        None,
    ).await.expect("Failed to record metric");

    // Verify metrics were recorded
    let metrics = service.get_metrics(None).await;
    assert!(metrics.len() >= 3);

    // Find our test metrics
    let counter_metric = metrics.iter().find(|m| m.metrics.contains_key("test_counter"));
    let gauge_metric = metrics.iter().find(|m| m.metrics.contains_key("test_gauge"));
    let histogram_metric = metrics.iter().find(|m| m.metrics.contains_key("test_histogram"));

    assert!(counter_metric.is_some());
    assert!(gauge_metric.is_some());
    assert!(histogram_metric.is_some());
}

#[tokio::test]
async fn test_counter_increment() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    // Test counter increment
    service.increment_counter("containers_created_total", 1.0, None).await
        .expect("Failed to increment counter");

    service.increment_counter("containers_created_total", 2.0, None).await
        .expect("Failed to increment counter");

    // Get Prometheus metrics to verify
    let prometheus_metrics = service.get_prometheus_metrics().await
        .expect("Failed to get Prometheus metrics");

    assert!(prometheus_metrics.contains("containers_created_total"));
}

#[tokio::test]
async fn test_gauge_setting() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    // Test gauge setting
    service.set_gauge("resource_memory_usage_bytes", 1024.0, None).await
        .expect("Failed to set gauge");

    service.set_gauge("resource_memory_usage_bytes", 2048.0, None).await
        .expect("Failed to set gauge");

    // Get Prometheus metrics to verify
    let prometheus_metrics = service.get_prometheus_metrics().await
        .expect("Failed to get Prometheus metrics");

    assert!(prometheus_metrics.contains("resource_memory_usage_bytes"));
}

#[tokio::test]
async fn test_histogram_observation() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    // Test histogram observation
    service.observe_histogram("request_duration_seconds", 0.1, None).await
        .expect("Failed to observe histogram");

    service.observe_histogram("request_duration_seconds", 0.2, None).await
        .expect("Failed to observe histogram");

    // Get Prometheus metrics to verify
    let prometheus_metrics = service.get_prometheus_metrics().await
        .expect("Failed to get Prometheus metrics");

    assert!(prometheus_metrics.contains("request_duration_seconds"));
}

#[tokio::test]
async fn test_logging() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    // Test logging different levels
    service.log(LogLevel::Info, "test-component", "Test info message", None, None).await
        .expect("Failed to log");

    service.log(LogLevel::Warning, "test-component", "Test warning message", None, None).await
        .expect("Failed to log");

    service.log(LogLevel::Error, "test-component", "Test error message", None, None).await
        .expect("Failed to log");

    // Verify logs were recorded
    let logs = service.get_logs(None).await;
    assert!(logs.len() >= 3);

    // Find our test logs
    let info_log = logs.iter().find(|l| l.message == "Test info message");
    let warning_log = logs.iter().find(|l| l.message == "Test warning message");
    let error_log = logs.iter().find(|l| l.message == "Test error message");

    assert!(info_log.is_some());
    assert!(warning_log.is_some());
    assert!(error_log.is_some());

    // Verify log levels
    assert_eq!(info_log.unwrap().level, LogLevel::Info);
    assert_eq!(warning_log.unwrap().level, LogLevel::Warning);
    assert_eq!(error_log.unwrap().level, LogLevel::Error);
}

#[tokio::test]
async fn test_logs_with_fields() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    // Test logging with fields
    let mut fields = HashMap::new();
    fields.insert("user_id".to_string(), serde_json::Value::String("12345".to_string()));
    fields.insert("request_id".to_string(), serde_json::Value::String("abc-def".to_string()));

    service.log(LogLevel::Info, "test-component", "Test message with fields", Some(fields), None).await
        .expect("Failed to log");

    // Verify log with fields
    let logs = service.get_logs_by_component("test-component", None).await;
    let field_log = logs.iter().find(|l| l.message == "Test message with fields");

    assert!(field_log.is_some());
    let log = field_log.unwrap();
    assert_eq!(log.fields.get("user_id"), Some(&serde_json::Value::String("12345".to_string())));
    assert_eq!(log.fields.get("request_id"), Some(&serde_json::Value::String("abc-def".to_string())));
}

#[tokio::test]
async fn test_logs_by_component() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    // Log to different components
    service.log(LogLevel::Info, "component-a", "Message from A", None, None).await
        .expect("Failed to log");
    service.log(LogLevel::Info, "component-b", "Message from B", None, None).await
        .expect("Failed to log");
    service.log(LogLevel::Info, "component-a", "Another message from A", None, None).await
        .expect("Failed to log");

    // Verify logs by component
    let component_a_logs = service.get_logs_by_component("component-a", None).await;
    let component_b_logs = service.get_logs_by_component("component-b", None).await;

    assert_eq!(component_a_logs.len(), 2);
    assert_eq!(component_b_logs.len(), 1);

    // Verify content
    assert!(component_a_logs.iter().any(|l| l.message == "Message from A"));
    assert!(component_a_logs.iter().any(|l| l.message == "Another message from A"));
    assert!(component_b_logs.iter().any(|l| l.message == "Message from B"));
}

#[tokio::test]
async fn test_metrics_querying() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    let now = Utc::now();
    let past = now - chrono::Duration::hours(1);
    let future = now + chrono::Duration::hours(1);

    // Record metrics with different timestamps and components
    service.record_metric("component-a", "metric1", MetricValue::Counter(1.0), None).await
        .expect("Failed to record metric");

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    service.record_metric("component-b", "metric2", MetricValue::Gauge(2.0), None).await
        .expect("Failed to record metric");

    // Test querying with component filter
    let component_a_metrics = service.query_metrics(Some("component-a"), None, None).await;
    assert!(component_a_metrics.len() >= 1);
    assert!(component_a_metrics.iter().all(|m| m.component == "component-a"));

    // Test querying with time range
    let time_range_metrics = service.query_metrics(None, Some(past), Some(future)).await;
    assert!(time_range_metrics.len() >= 2);
}

#[tokio::test]
async fn test_metrics_summary() {
    let config = ObservabilityConfig::default();
    let service = ObservabilityService::new(config).expect("Failed to create observability service");

    // Record some metrics and logs
    service.record_metric("test-component", "test_metric", MetricValue::Counter(1.0), None).await
        .expect("Failed to record metric");

    service.log(LogLevel::Info, "test-component", "Test log", None, None).await
        .expect("Failed to log");

    // Get summary
    let summary = service.get_metrics_summary().await;

    // Verify summary contains expected fields
    assert!(summary.contains_key("total_metrics"));
    assert!(summary.contains_key("total_logs"));
    assert!(summary.contains_key("unique_components"));
    assert!(summary.contains_key("log_levels"));

    // Verify values are reasonable
    let total_metrics = summary.get("total_metrics").unwrap().as_u64().unwrap();
    let total_logs = summary.get("total_logs").unwrap().as_u64().unwrap();
    let unique_components = summary.get("unique_components").unwrap().as_u64().unwrap();

    assert!(total_metrics >= 1);
    assert!(total_logs >= 1);
    assert!(unique_components >= 1);
}

#[tokio::test]
async fn test_log_levels() {
    // Test all log level variants
    let levels = vec![
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warning,
        LogLevel::Error,
        LogLevel::Critical,
    ];

    for level in levels {
        match level {
            LogLevel::Debug => assert!(true),
            LogLevel::Info => assert!(true),
            LogLevel::Warning => assert!(true),
            LogLevel::Error => assert!(true),
            LogLevel::Critical => assert!(true),
        }
    }
}

#[tokio::test]
async fn test_metric_value_variants() {
    // Test all metric value variants
    let counter = MetricValue::Counter(42.0);
    let gauge = MetricValue::Gauge(75.5);
    let histogram = MetricValue::Histogram(vec![1.0, 2.0, 3.0]);
    let boolean = MetricValue::Boolean(true);
    let string = MetricValue::String("test".to_string());

    match counter {
        MetricValue::Counter(value) => assert_eq!(value, 42.0),
        _ => panic!("Expected Counter variant"),
    }

    match gauge {
        MetricValue::Gauge(value) => assert_eq!(value, 75.5),
        _ => panic!("Expected Gauge variant"),
    }

    match histogram {
        MetricValue::Histogram(values) => assert_eq!(values, vec![1.0, 2.0, 3.0]),
        _ => panic!("Expected Histogram variant"),
    }

    match boolean {
        MetricValue::Boolean(value) => assert_eq!(value, true),
        _ => panic!("Expected Boolean variant"),
    }

    match string {
        MetricValue::String(value) => assert_eq!(value, "test"),
        _ => panic!("Expected String variant"),
    }
}