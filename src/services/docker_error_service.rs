use crate::models::docker_config::{DockerConfigError, DockerContainerConfig};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, warn, info, debug, span, Level};

/// Service for handling Docker operation errors and logging
#[derive(Clone)]
pub struct DockerErrorService {
    /// Store for error logs
    error_logs: Arc<RwLock<HashMap<uuid::Uuid, Vec<DockerErrorLog>>>>,

    /// Store for operation metrics
    operation_metrics: Arc<RwLock<HashMap<String, OperationMetric>>>,

    /// Store for retry policies
    retry_policies: Arc<RwLock<HashMap<String, RetryPolicy>>>,
}

/// Individual error log entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DockerErrorLog {
    pub id: uuid::Uuid,
    pub operation: String,
    pub error_type: String,
    pub message: String,
    pub details: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub severity: ErrorSeverity,
    pub context_id: Option<uuid::Uuid>,
    pub retry_count: u32,
    pub resolved: bool,
}

/// Operation metric for performance monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperationMetric {
    pub operation: String,
    pub total_attempts: u64,
    pub successful_attempts: u64,
    pub failed_attempts: u64,
    pub average_duration_ms: f64,
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
    pub success_rate: f64,
}

/// Retry policy configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetryPolicy {
    pub operation: String,
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub retryable_errors: Vec<String>,
}

/// Error severity levels
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl DockerErrorService {
    /// Create a new Docker error service
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let service = Self {
            error_logs: Arc::new(RwLock::new(HashMap::new())),
            operation_metrics: Arc::new(RwLock::new(HashMap::new())),
            retry_policies: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize default retry policies
        service.initialize_default_policies().await?;

        Ok(service)
    }

    /// Initialize default retry policies
    async fn initialize_default_policies(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut policies = self.retry_policies.write().await;

        policies.insert("build".to_string(), RetryPolicy {
            operation: "build".to_string(),
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            retryable_errors: vec![
                "network_error".to_string(),
                "resource_temporarily_unavailable".to_string(),
                "timeout".to_string(),
            ],
        });

        policies.insert("scan".to_string(), RetryPolicy {
            operation: "scan".to_string(),
            max_retries: 2,
            initial_delay_ms: 2000,
            max_delay_ms: 60000,
            backoff_multiplier: 2.0,
            retryable_errors: vec![
                "network_error".to_string(),
                "scanner_unavailable".to_string(),
                "timeout".to_string(),
            ],
        });

        policies.insert("deploy".to_string(), RetryPolicy {
            operation: "deploy".to_string(),
            max_retries: 5,
            initial_delay_ms: 5000,
            max_delay_ms: 120000,
            backoff_multiplier: 2.0,
            retryable_errors: vec![
                "network_error".to_string(),
                "container_not_ready".to_string(),
                "health_check_failed".to_string(),
                "resource_constraint".to_string(),
            ],
        });

        Ok(())
    }

    /// Log an error with full context
    pub async fn log_error(
        &self,
        operation: String,
        error_type: String,
        message: String,
        details: HashMap<String, String>,
        context_id: Option<uuid::Uuid>,
        severity: ErrorSeverity,
    ) -> Result<uuid::Uuid, DockerConfigError> {
        let error_id = uuid::Uuid::new_v4();
        let timestamp = chrono::Utc::now();

        let error_log = DockerErrorLog {
            id: error_id,
            operation: operation.clone(),
            error_type: error_type.clone(),
            message: message.clone(),
            details,
            timestamp,
            severity: severity.clone(),
            context_id,
            retry_count: 0,
            resolved: false,
        };

        // Log to structured logging
        match severity {
            ErrorSeverity::Critical => {
                error!(
                    operation = %operation,
                    error_type = %error_type,
                    message = %message,
                    context_id = ?context_id,
                    "[DOCKER_CRITICAL] {} - {}: {}", operation, error_type, message
                );
            }
            ErrorSeverity::High => {
                error!(
                    operation = %operation,
                    error_type = %error_type,
                    message = %message,
                    context_id = ?context_id,
                    "[DOCKER_ERROR] {} - {}: {}", operation, error_type, message
                );
            }
            ErrorSeverity::Medium => {
                warn!(
                    operation = %operation,
                    error_type = %error_type,
                    message = %message,
                    context_id = ?context_id,
                    "[DOCKER_WARN] {} - {}: {}", operation, error_type, message
                );
            }
            ErrorSeverity::Low => {
                info!(
                    operation = %operation,
                    error_type = %error_type,
                    message = %message,
                    context_id = ?context_id,
                    "[DOCKER_INFO] {} - {}: {}", operation, error_type, message
                );
            }
        }

        // Store error log
        let mut logs = self.error_logs.write().await;
        logs.entry(context_id.unwrap_or(uuid::Uuid::nil()))
            .or_insert_with(Vec::new)
            .push(error_log);

        // Update operation metrics
        self.update_operation_metrics(&operation, false, None).await?;

        Ok(error_id)
    }

    /// Log successful operation
    pub async fn log_success(
        &self,
        operation: String,
        duration_ms: u64,
        context_id: Option<uuid::Uuid>,
    ) -> Result<(), DockerConfigError> {
        let span = span!(Level::INFO, "docker_operation", operation = %operation);
        let _enter = span.enter();

        info!(
            operation = %operation,
            duration_ms = duration_ms,
            context_id = ?context_id,
            "[DOCKER_SUCCESS] {} completed in {}ms", operation, duration_ms
        );

        // Update operation metrics
        self.update_operation_metrics(&operation, true, Some(duration_ms)).await?;

        Ok(())
    }

    /// Update operation metrics
    async fn update_operation_metrics(
        &self,
        operation: &str,
        success: bool,
        duration_ms: Option<u64>,
    ) -> Result<(), DockerConfigError> {
        let mut metrics = self.operation_metrics.write().await;
        let metric = metrics.entry(operation.to_string()).or_insert_with(|| OperationMetric {
            operation: operation.to_string(),
            total_attempts: 0,
            successful_attempts: 0,
            failed_attempts: 0,
            average_duration_ms: 0.0,
            last_success: None,
            last_failure: None,
            success_rate: 0.0,
        });

        metric.total_attempts += 1;

        if success {
            metric.successful_attempts += 1;
            metric.last_success = Some(chrono::Utc::now());

            if let Some(duration) = duration_ms {
                // Update average duration
                metric.average_duration_ms =
                    (metric.average_duration_ms * (metric.successful_attempts - 1) as f64 + duration as f64)
                    / metric.successful_attempts as f64;
            }
        } else {
            metric.failed_attempts += 1;
            metric.last_failure = Some(chrono::Utc::now());
        }

        // Calculate success rate
        metric.success_rate = if metric.total_attempts > 0 {
            metric.successful_attempts as f64 / metric.total_attempts as f64
        } else {
            0.0
        };

        Ok(())
    }

    /// Get retry policy for operation
    pub async fn get_retry_policy(&self, operation: &str) -> Option<RetryPolicy> {
        let policies = self.retry_policies.read().await;
        policies.get(operation).cloned()
    }

    /// Check if error is retryable
    pub async fn is_retryable_error(&self, operation: &str, error_type: &str) -> bool {
        if let Some(policy) = self.get_retry_policy(operation).await {
            policy.retryable_errors.contains(&error_type.to_string())
        } else {
            false
        }
    }

    /// Calculate retry delay with exponential backoff
    pub async fn calculate_retry_delay(&self, operation: &str, retry_count: u32) -> Option<u64> {
        if let Some(policy) = self.get_retry_policy(operation).await {
            if retry_count >= policy.max_retries {
                return None;
            }

            let delay = (policy.initial_delay_ms as f64 *
                        policy.backoff_multiplier.powi(retry_count as i32))
                .min(policy.max_delay_ms as f64) as u64;

            Some(delay)
        } else {
            None
        }
    }

    /// Get error logs for context
    pub async fn get_error_logs(&self, context_id: Option<uuid::Uuid>) -> Result<Vec<DockerErrorLog>, DockerConfigError> {
        let logs = self.error_logs.read().await;
        Ok(logs.get(&context_id.unwrap_or(uuid::Uuid::nil()))
            .cloned()
            .unwrap_or_else(Vec::new))
    }

    /// Get operation metrics
    pub async fn get_operation_metrics(&self) -> Result<HashMap<String, OperationMetric>, DockerConfigError> {
        let metrics = self.operation_metrics.read().await;
        Ok(metrics.clone())
    }

    /// Get error summary
    pub async fn get_error_summary(&self) -> Result<serde_json::Value, DockerConfigError> {
        let logs = self.error_logs.read().await;
        let metrics = self.operation_metrics.read().await;

        let total_errors: usize = logs.values().map(|v| v.len()).sum();
        let critical_errors: usize = logs.values()
            .flat_map(|v| v.iter())
            .filter(|log| log.severity == ErrorSeverity::Critical)
            .count();

        let unresolved_errors: usize = logs.values()
            .flat_map(|v| v.iter())
            .filter(|log| !log.resolved)
            .count();

        let operations_count = metrics.len();
        let overall_success_rate = if operations_count > 0 {
            metrics.values()
                .map(|m| m.success_rate)
                .sum::<f64>() / operations_count as f64
        } else {
            0.0
        };

        Ok(json!({
            "total_errors": total_errors,
            "critical_errors": critical_errors,
            "unresolved_errors": unresolved_errors,
            "operations_monitored": operations_count,
            "overall_success_rate": overall_success_rate,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Mark error as resolved
    pub async fn resolve_error(&self, error_id: uuid::Uuid) -> Result<bool, DockerConfigError> {
        let mut logs = self.error_logs.write().await;

        for context_logs in logs.values_mut() {
            if let Some(error_log) = context_logs.iter_mut().find(|log| log.id == error_id) {
                error_log.resolved = true;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Retry operation with exponential backoff
    pub async fn retry_operation<F, Fut, T>(
        &self,
        operation: String,
        context_id: Option<uuid::Uuid>,
        mut operation_func: F,
    ) -> Result<T, DockerConfigError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, DockerConfigError>>,
    {
        let mut retry_count = 0;
        let start_time = std::time::Instant::now();

        loop {
            match operation_func().await {
                Ok(result) => {
                    let duration = start_time.elapsed().as_millis() as u64;
                    self.log_success(operation.clone(), duration, context_id).await?;
                    return Ok(result);
                }
                Err(e) => {
                    let error_type = match e {
                        DockerConfigError::ValidationError(_) => "validation_error",
                        DockerConfigError::InvalidDockerfile(_) => "build_error",
                        DockerConfigError::SecurityViolation(_) => "security_error",
                        DockerConfigError::SecurityError(_) => "security_error",
                        DockerConfigError::ResourceLimitsError(_) => "resource_error",
                        DockerConfigError::InvalidImageName(_) => "image_error",
                        DockerConfigError::DockerfileNotFound(_) => "file_not_found",
                        DockerConfigError::Validation(_) => "validation_error",
                        DockerConfigError::BuildError(_) => "build_error",
                    };

                    // Log the error
                    let error_id = self.log_error(
                        operation.clone(),
                        error_type.to_string(),
                        e.to_string(),
                        HashMap::new(),
                        context_id,
                        ErrorSeverity::High,
                    ).await?;

                    // Check if we should retry
                    if self.is_retryable_error(&operation, error_type).await {
                        if let Some(delay) = self.calculate_retry_delay(&operation, retry_count).await {
                            retry_count += 1;

                            // Update retry count in error log
                            let mut logs = self.error_logs.write().await;
                            if let Some(context_logs) = logs.get_mut(&context_id.unwrap_or(uuid::Uuid::nil())) {
                                if let Some(error_log) = context_logs.iter_mut().find(|log| log.id == error_id) {
                                    error_log.retry_count = retry_count;
                                }
                            }

                            warn!(
                                operation = %operation,
                                retry_count = retry_count,
                                delay_ms = delay,
                                "Retrying operation after {}ms (attempt {}/{})",
                                delay, retry_count, self.get_retry_policy(&operation).await.unwrap().max_retries
                            );

                            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                            continue;
                        }
                    }

                    // Max retries reached or error not retryable
                    error!(
                        operation = %operation,
                        retry_count = retry_count,
                        "Operation failed after {} retries", retry_count
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Execute operation with error handling and logging
    pub async fn execute_operation<F, Fut, T>(
        &self,
        operation: String,
        context_id: Option<uuid::Uuid>,
        operation_func: F,
    ) -> Result<T, DockerConfigError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, DockerConfigError>>,
    {
        let start_time = std::time::Instant::now();
        let span = span!(Level::INFO, "docker_operation", operation = %operation);
        let _enter = span.enter();

        debug!("Starting operation: {}", operation);

        match operation_func().await {
            Ok(result) => {
                let duration = start_time.elapsed().as_millis() as u64;
                self.log_success(operation, duration, context_id).await?;
                Ok(result)
            }
            Err(e) => {
                let duration = start_time.elapsed().as_millis() as u64;

                let error_type = match e {
                    DockerConfigError::ValidationError(_) => "validation_error",
                    DockerConfigError::InvalidDockerfile(_) => "build_error",
                    DockerConfigError::SecurityViolation(_) => "security_error",
                    DockerConfigError::SecurityError(_) => "security_error",
                    DockerConfigError::ResourceLimitsError(_) => "resource_error",
                    DockerConfigError::InvalidImageName(_) => "image_error",
                    DockerConfigError::DockerfileNotFound(_) => "file_not_found",
                    DockerConfigError::Validation(_) => "validation_error",
                    DockerConfigError::BuildError(_) => "build_error",
                };

                let mut details = HashMap::new();
                details.insert("duration_ms".to_string(), duration.to_string());

                self.log_error(
                    operation,
                    error_type.to_string(),
                    e.to_string(),
                    details,
                    context_id,
                    ErrorSeverity::High,
                ).await?;

                Err(e)
            }
        }
    }

    /// Health check for error service
    pub async fn health_check(&self) -> serde_json::Value {
        let metrics = self.operation_metrics.read().await;
        let logs = self.error_logs.read().await;

        let total_operations = metrics.len();
        let healthy_operations = metrics.values()
            .filter(|m| m.success_rate > 0.8)
            .count();

        let recent_errors = logs.values()
            .flat_map(|v| v.iter())
            .filter(|log| {
                let time_diff = chrono::Utc::now().signed_duration_since(log.timestamp);
                time_diff.num_minutes() < 30
            })
            .count();

        json!({
            "status": if healthy_operations as f64 / total_operations.max(1) as f64 > 0.8 && recent_errors < 10 {
                "healthy"
            } else {
                "degraded"
            },
            "total_operations": total_operations,
            "healthy_operations": healthy_operations,
            "recent_errors": recent_errors,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_service() {
        let service = DockerErrorService::new().await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_log_error() {
        let service = DockerErrorService::new().await.unwrap();
        let result = service.log_error(
            "test_operation".to_string(),
            "test_error".to_string(),
            "Test error message".to_string(),
            HashMap::new(),
            None,
            ErrorSeverity::Medium,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_log_success() {
        let service = DockerErrorService::new().await.unwrap();
        let result = service.log_success(
            "test_operation".to_string(),
            100,
            None,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_retry_policy() {
        let service = DockerErrorService::new().unwrap();
        let policy = service.get_retry_policy("build").await;

        assert!(policy.is_some());
        assert_eq!(policy.unwrap().operation, "build");
    }

    #[tokio::test]
    async fn test_is_retryable_error() {
        let service = DockerErrorService::new().unwrap();
        let retryable = service.is_retryable_error("build", "network_error").await;
        let not_retryable = service.is_retryable_error("build", "unknown_error").await;

        assert!(retryable);
        assert!(!not_retryable);
    }

    #[tokio::test]
    async fn test_calculate_retry_delay() {
        let service = DockerErrorService::new().unwrap();
        let delay = service.calculate_retry_delay("build", 1).await;

        assert!(delay.is_some());
        assert_eq!(delay.unwrap(), 2000); // 1000ms * 2.0^1
    }

    #[tokio::test]
    async fn test_execute_operation_success() {
        let service = DockerErrorService::new().await.unwrap();

        let result = service.execute_operation(
            "test_operation".to_string(),
            None,
            || async { Ok::<(), DockerConfigError>(()) },
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_operation_failure() {
        let service = DockerErrorService::new().await.unwrap();

        let result = service.execute_operation(
            "test_operation".to_string(),
            None,
            || async { Err::<(), DockerConfigError>(DockerConfigError::ValidationError("Test error".to_string())) },
        ).await;

        assert!(result.is_err());
    }
}