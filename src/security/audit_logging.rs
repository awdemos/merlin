//! Audit logging module for Merlin AI Router Docker deployment
//! Provides comprehensive audit logging for security and compliance

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::models::container_config::DockerContainerConfig;
use super::access_control::{User, Permission};
use super::docker_client::DockerConfigError;

/// Audit event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    ConfigurationChange,
    ContainerOperation,
    SecurityScan,
    PolicyViolation,
    Deployment,
    SystemOperation,
    DataAccess,
    AdminAction,
    Compliance,
    Custom(String),
}

/// Audit event severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Audit event status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditStatus {
    Success,
    Failure,
    Attempt,
    Blocked,
    Warning,
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub severity: AuditSeverity,
    pub status: AuditStatus,
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub session_id: Option<Uuid>,
    pub resource: String,
    pub action: String,
    pub details: HashMap<String, serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub parent_event_id: Option<Uuid>,
    pub duration_ms: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub compliance_tags: Vec<String>,
}

/// Audit log query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub event_types: Option<Vec<AuditEventType>>,
    pub severities: Option<Vec<AuditSeverity>>,
    pub statuses: Option<Vec<AuditStatus>>,
    pub user_ids: Option<Vec<Uuid>>,
    pub usernames: Option<Vec<String>>,
    pub resources: Option<Vec<String>>,
    pub actions: Option<Vec<String>>,
    pub ip_addresses: Option<Vec<String>>,
    pub session_ids: Option<Vec<Uuid>>,
    pub correlation_ids: Option<Vec<Uuid>>,
    pub compliance_tags: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_field: Option<String>,
    pub sort_direction: Option<SortDirection>,
}

/// Sort direction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Audit log statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_events: u64,
    pub events_by_type: HashMap<AuditEventType, u64>,
    pub events_by_severity: HashMap<AuditSeverity, u64>,
    pub events_by_status: HashMap<AuditStatus, u64>,
    pub events_by_user: HashMap<String, u64>,
    pub top_resources: Vec<(String, u64)>,
    pub top_actions: Vec<(String, u64)>,
    pub time_range: (DateTime<Utc>, DateTime<Utc>),
}

/// Audit retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub default_retention_days: u64,
    pub event_type_retention: HashMap<AuditEventType, u64>,
    pub severity_retention: HashMap<AuditSeverity, u64>,
    pub max_storage_size_gb: Option<u64>,
    pub enable_compression: bool,
    pub enable_archival: bool,
    pub archival_threshold_days: u64,
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub retention_policy: RetentionPolicy,
    pub enable_real_time_alerts: bool,
    pub alert_thresholds: AlertThresholds,
    pub required_fields: Vec<String>,
    pub sensitive_fields: Vec<String>,
    pub compliance_requirements: Vec<ComplianceRequirement>,
}

/// Alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub failed_authentications_per_minute: u32,
    pub failed_authorizations_per_minute: u32,
    pub critical_events_per_hour: u32,
    pub policy_violations_per_hour: u32,
    pub unusual_activity_score: f64,
}

/// Compliance requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequirement {
    pub name: String,
    pub description: String,
    pub required_fields: Vec<String>,
    pub retention_days: u64,
    pub event_types: Vec<AuditEventType>,
    pub alert_on_violation: bool,
}

/// Audit logging service
#[derive(Clone)]
pub struct AuditService {
    events: Arc<RwLock<Vec<AuditEvent>>>,
    config: AuditConfig,
    retention_policy: RetentionPolicy,
    alerts: Arc<RwLock<Vec<AuditAlert>>>,
    real_time_subscribers: Arc<RwLock<Vec<AuditSubscriber>>>,
    compliance_checker: ComplianceChecker,
}

/// Audit alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditAlert {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub severity: AlertSeverity,
    pub alert_type: AlertType,
    pub message: String,
    pub details: HashMap<String, serde_json::Value>,
    pub event_ids: Vec<Uuid>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Alert severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Alert type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertType {
    AuthenticationFailure,
    AuthorizationFailure,
    PolicyViolation,
    SecurityEvent,
    ComplianceViolation,
    SystemError,
    UnusualActivity,
}

/// Audit subscriber for real-time notifications
#[derive(Debug, Clone)]
pub struct AuditSubscriber {
    pub id: Uuid,
    pub callback: Box<dyn Fn(AuditEvent) + Send + Sync>,
    pub filter: Option<AuditQuery>,
}

/// Compliance checker
#[derive(Debug, Clone)]
pub struct ComplianceChecker {
    requirements: Vec<ComplianceRequirement>,
}

impl AuditService {
    pub fn new(config: AuditConfig) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            retention_policy: config.retention_policy.clone(),
            config,
            alerts: Arc::new(RwLock::new(Vec::new())),
            real_time_subscribers: Arc::new(RwLock::new(Vec::new())),
            compliance_checker: ComplianceChecker::new(),
        }
    }

    /// Log an audit event
    pub async fn log_event(&self, event: AuditEvent) -> Result<(), DockerConfigError> {
        if !self.config.enabled {
            return Ok(());
        }

        // Validate required fields
        self.validate_event(&event)?;

        // Store event
        let mut events = self.events.write().await;
        events.push(event.clone());

        // Apply retention policy
        self.apply_retention_policy(&mut events).await;

        // Send real-time notifications
        self.notify_subscribers(&event).await;

        // Check for alerts
        self.check_alerts(&event).await;

        // Check compliance
        self.check_compliance(&event).await;

        Ok(())
    }

    /// Log authentication event
    pub async fn log_authentication(
        &self,
        username: &str,
        user_id: Option<Uuid>,
        success: bool,
        ip_address: Option<String>,
        user_agent: Option<String>,
        session_id: Option<Uuid>,
        error_message: Option<String>,
    ) -> Result<(), DockerConfigError> {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::Authentication,
            severity: if success { AuditSeverity::Info } else { AuditSeverity::Warning },
            status: if success { AuditStatus::Success } else { AuditStatus::Failure },
            user_id,
            username: Some(username.to_string()),
            session_id,
            resource: "authentication".to_string(),
            action: if success { "login" } else { "login_failed" }.to_string(),
            details: {
                let mut details = HashMap::new();
                details.insert("username".to_string(), serde_json::Value::String(username.to_string()));
                if let Some(error) = &error_message {
                    details.insert("error".to_string(), serde_json::Value::String(error.clone()));
                }
                details
            },
            ip_address,
            user_agent,
            correlation_id: None,
            parent_event_id: None,
            duration_ms: None,
            result: Some(serde_json::Value::Bool(success)),
            error_message,
            compliance_tags: vec!["authentication".to_string(), "security".to_string()],
        };

        self.log_event(event).await
    }

    /// Log authorization event
    pub async fn log_authorization(
        &self,
        user: &User,
        permission: &Permission,
        resource: &str,
        allowed: bool,
        ip_address: Option<String>,
        user_agent: Option<String>,
        session_id: Option<Uuid>,
        reason: Option<String>,
    ) -> Result<(), DockerConfigError> {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::Authorization,
            severity: if allowed { AuditSeverity::Info } else { AuditSeverity::Warning },
            status: if allowed { AuditStatus::Success } else { AuditStatus::Blocked },
            user_id: Some(user.id),
            username: Some(user.username.clone()),
            session_id,
            resource: resource.to_string(),
            action: format!("{:?}", permission),
            details: {
                let mut details = HashMap::new();
                details.insert("user_role".to_string(), serde_json::Value::String(format!("{:?}", user.role)));
                details.insert("permission".to_string(), serde_json::Value::String(format!("{:?}", permission)));
                if let Some(r) = &reason {
                    details.insert("reason".to_string(), serde_json::Value::String(r.clone()));
                }
                details
            },
            ip_address,
            user_agent,
            correlation_id: None,
            parent_event_id: None,
            duration_ms: None,
            result: Some(serde_json::Value::Bool(allowed)),
            error_message: reason,
            compliance_tags: vec!["authorization".to_string(), "security".to_string()],
        };

        self.log_event(event).await
    }

    /// Log container operation
    pub async fn log_container_operation(
        &self,
        user_id: Option<Uuid>,
        username: Option<String>,
        operation: &str,
        container_id: &str,
        success: bool,
        details: HashMap<String, serde_json::Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        session_id: Option<Uuid>,
        error_message: Option<String>,
    ) -> Result<(), DockerConfigError> {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::ContainerOperation,
            severity: if success { AuditSeverity::Info } else { AuditSeverity::Error },
            status: if success { AuditStatus::Success } else { AuditStatus::Failure },
            user_id,
            username,
            session_id,
            resource: format!("container:{}", container_id),
            action: operation.to_string(),
            details,
            ip_address,
            user_agent,
            correlation_id: None,
            parent_event_id: None,
            duration_ms: None,
            result: Some(serde_json::Value::Bool(success)),
            error_message,
            compliance_tags: vec!["container".to_string(), "operations".to_string()],
        };

        self.log_event(event).await
    }

    /// Log security scan
    pub async fn log_security_scan(
        &self,
        user_id: Option<Uuid>,
        username: Option<String>,
        scan_type: &str,
        target: &str,
        vulnerabilities_found: u32,
        compliance_score: f64,
        details: HashMap<String, serde_json::Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        session_id: Option<Uuid>,
    ) -> Result<(), DockerConfigError> {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::SecurityScan,
            severity: if vulnerabilities_found > 0 { AuditSeverity::Warning } else { AuditSeverity::Info },
            status: AuditStatus::Success,
            user_id,
            username,
            session_id,
            resource: target.to_string(),
            action: scan_type.to_string(),
            details: {
                let mut scan_details = details;
                scan_details.insert("vulnerabilities_found".to_string(), serde_json::Value::Number(serde_json::Number::from(vulnerabilities_found)));
                scan_details.insert("compliance_score".to_string(), serde_json::Value::Number(serde_json::Number::from(compliance_score)));
                scan_details
            },
            ip_address,
            user_agent,
            correlation_id: None,
            parent_event_id: None,
            duration_ms: None,
            result: Some(serde_json::Value::Object({
                let mut result = HashMap::new();
                result.insert("vulnerabilities_found".to_string(), serde_json::Value::Number(serde_json::Number::from(vulnerabilities_found)));
                result.insert("compliance_score".to_string(), serde_json::Value::Number(serde_json::Number::from(compliance_score)));
                result
            })),
            error_message: None,
            compliance_tags: vec!["security".to_string(), "compliance".to_string(), "scanning".to_string()],
        };

        self.log_event(event).await
    }

    /// Query audit events
    pub async fn query_events(&self, query: AuditQuery) -> Result<Vec<AuditEvent>, DockerConfigError> {
        let events = self.events.read().await;

        let filtered_events: Vec<AuditEvent> = events
            .iter()
            .filter(|event| self.matches_query(event, &query))
            .cloned()
            .collect();

        // Apply sorting
        let mut sorted_events = filtered_events;
        if let Some(sort_field) = &query.sort_field {
            self.sort_events(&mut sorted_events, sort_field, query.sort_direction.as_ref().unwrap_or(&SortDirection::Desc));
        }

        // Apply pagination
        let start = query.offset.unwrap_or(0);
        let end = start + query.limit.unwrap_or(sorted_events.len());
        let paginated_events = sorted_events[start..end.min(sorted_events.len())].to_vec();

        Ok(paginated_events)
    }

    /// Get audit statistics
    pub async fn get_statistics(&self, query: Option<AuditQuery>) -> Result<AuditStatistics, DockerConfigError> {
        let events = if let Some(q) = query {
            self.query_events(q).await?
        } else {
            let events = self.events.read().await;
            events.clone()
        };

        let mut stats = AuditStatistics {
            total_events: events.len() as u64,
            events_by_type: HashMap::new(),
            events_by_severity: HashMap::new(),
            events_by_status: HashMap::new(),
            events_by_user: HashMap::new(),
            top_resources: Vec::new(),
            top_actions: Vec::new(),
            time_range: if events.is_empty() {
                (Utc::now(), Utc::now())
            } else {
                let timestamps: Vec<_> = events.iter().map(|e| e.timestamp).collect();
                (*timestamps.first().unwrap(), *timestamps.last().unwrap())
            },
        };

        // Count by various dimensions
        for event in &events {
            // By type
            *stats.events_by_type.entry(event.event_type.clone()).or_insert(0) += 1;

            // By severity
            *stats.events_by_severity.entry(event.severity.clone()).or_insert(0) += 1;

            // By status
            *stats.events_by_status.entry(event.status.clone()).or_insert(0) += 1;

            // By user
            if let Some(username) = &event.username {
                *stats.events_by_user.entry(username.clone()).or_insert(0) += 1;
            }

            // Resources and actions
            *stats.top_resources.entry(event.resource.clone()).or_insert(0) += 1;
            *stats.top_actions.entry(event.action.clone()).or_insert(0) += 1;
        }

        // Sort top resources and actions
        stats.top_resources.sort_by(|a, b| b.1.cmp(&a.1));
        stats.top_actions.sort_by(|a, b| b.1.cmp(&a.1));

        // Keep top 10
        stats.top_resources.truncate(10);
        stats.top_actions.truncate(10);

        Ok(stats)
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<AuditAlert> {
        let alerts = self.alerts.read().await;
        alerts.iter().filter(|a| !a.resolved).cloned().collect()
    }

    /// Resolve alert
    pub async fn resolve_alert(&self, alert_id: Uuid) -> Result<(), DockerConfigError> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.resolved = true;
            alert.resolved_at = Some(Utc::now());
            Ok(())
        } else {
            Err(DockerConfigError::Validation("Alert not found".to_string()))
        }
    }

    /// Subscribe to real-time audit events
    pub async fn subscribe(&self, filter: Option<AuditQuery>) -> Uuid {
        let subscriber_id = Uuid::new_v4();
        let subscriber = AuditSubscriber {
            id: subscriber_id,
            callback: Box::new(|_event| {
                // In a real implementation, this would send notifications
                // For now, we'll just log to console
                println!("Audit event received");
            }),
            filter,
        };

        let mut subscribers = self.real_time_subscribers.write().await;
        subscribers.push(subscriber);

        subscriber_id
    }

    /// Unsubscribe from audit events
    pub async fn unsubscribe(&self, subscriber_id: Uuid) -> Result<(), DockerConfigError> {
        let mut subscribers = self.real_time_subscribers.write().await;
        subscribers.retain(|s| s.id != subscriber_id);
        Ok(())
    }

    /// Validate event fields
    fn validate_event(&self, event: &AuditEvent) -> Result<(), DockerConfigError> {
        for required_field in &self.config.required_fields {
            match required_field.as_str() {
                "user_id" => {
                    if event.user_id.is_none() && event.username.is_none() {
                        return Err(DockerConfigError::Validation("User identification required".to_string()));
                    }
                }
                "resource" => {
                    if event.resource.is_empty() {
                        return Err(DockerConfigError::Validation("Resource required".to_string()));
                    }
                }
                "action" => {
                    if event.action.is_empty() {
                        return Err(DockerConfigError::Validation("Action required".to_string()));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Apply retention policy
    async fn apply_retention_policy(&self, events: &mut Vec<AuditEvent>) {
        let now = Utc::now();
        events.retain(|event| {
            let retention_days = self.retention_policy.event_type_retention
                .get(&event.event_type)
                .copied()
                .unwrap_or(self.retention_policy.default_retention_days);

            let cutoff = now - chrono::Duration::days(retention_days as i64);
            event.timestamp > cutoff
        });

        // Keep only last 100,000 events
        if events.len() > 100_000 {
            *events = events[events.len() - 100_000..].to_vec();
        }
    }

    /// Notify subscribers of new events
    async fn notify_subscribers(&self, event: &AuditEvent) {
        let subscribers = self.real_time_subscribers.read().await;
        for subscriber in subscribers.iter() {
            if let Some(filter) = &subscriber.filter {
                if self.matches_query(event, filter) {
                    (subscriber.callback)(event.clone());
                }
            } else {
                (subscriber.callback)(event.clone());
            }
        }
    }

    /// Check for alert conditions
    async fn check_alerts(&self, event: &AuditEvent) {
        let thresholds = &self.config.alert_thresholds;

        // Check for authentication failures
        if event.event_type == AuditEventType::Authentication && event.status == AuditStatus::Failure {
            // Count recent authentication failures
            let recent_failures = self.count_recent_events(
                Some(AuditEventType::Authentication),
                Some(AuditStatus::Failure),
                60, // last minute
            ).await;

            if recent_failures >= thresholds.failed_authentications_per_minute {
                self.create_alert(
                    AlertType::AuthenticationFailure,
                    AlertSeverity::High,
                    format!("High rate of authentication failures: {} in last minute", recent_failures),
                    vec![event.id],
                ).await;
            }
        }

        // Check for authorization failures
        if event.event_type == AuditEventType::Authorization && event.status == AuditStatus::Blocked {
            let recent_authorization_failures = self.count_recent_events(
                Some(AuditEventType::Authorization),
                Some(AuditStatus::Blocked),
                60, // last minute
            ).await;

            if recent_authorization_failures >= thresholds.failed_authorizations_per_minute {
                self.create_alert(
                    AlertType::AuthorizationFailure,
                    AlertSeverity::High,
                    format!("High rate of authorization failures: {} in last minute", recent_authorization_failures),
                    vec![event.id],
                ).await;
            }
        }

        // Check for critical events
        if event.severity == AuditSeverity::Critical {
            let recent_critical_events = self.count_recent_events(
                None,
                None,
                3600, // last hour
            ).await;

            if recent_critical_events >= thresholds.critical_events_per_hour {
                self.create_alert(
                    AlertType::SecurityEvent,
                    AlertSeverity::Critical,
                    format!("High rate of critical events: {} in last hour", recent_critical_events),
                    vec![event.id],
                ).await;
            }
        }
    }

    /// Check compliance requirements
    async fn check_compliance(&self, event: &AuditEvent) {
        // Check each compliance requirement
        for requirement in &self.compliance_checker.requirements {
            if requirement.event_types.contains(&event.event_type) {
                // Check if all required fields are present
                let mut missing_fields = Vec::new();
                for field in &requirement.required_fields {
                    if !self.has_field(event, field) {
                        missing_fields.push(field.clone());
                    }
                }

                if !missing_fields.is_empty() && requirement.alert_on_violation {
                    self.create_alert(
                        AlertType::ComplianceViolation,
                        AlertSeverity::Medium,
                        format!("Compliance violation for {}: missing fields {:?}", requirement.name, missing_fields),
                        vec![event.id],
                    ).await;
                }
            }
        }
    }

    /// Create alert
    async fn create_alert(&self, alert_type: AlertType, severity: AlertSeverity, message: String, event_ids: Vec<Uuid>) {
        let alert = AuditAlert {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            severity,
            alert_type,
            message,
            details: HashMap::new(),
            event_ids,
            resolved: false,
            resolved_at: None,
        };

        let mut alerts = self.alerts.write().await;
        alerts.push(alert);

        // Keep only last 1000 alerts
        if alerts.len() > 1000 {
            *alerts = alerts[alerts.len() - 1000..].to_vec();
        }
    }

    /// Check if event matches query
    fn matches_query(&self, event: &AuditEvent, query: &AuditQuery) -> bool {
        // Time range
        if let Some(start) = query.start_time {
            if event.timestamp < start {
                return false;
            }
        }
        if let Some(end) = query.end_time {
            if event.timestamp > end {
                return false;
            }
        }

        // Event types
        if let Some(types) = &query.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }

        // Severities
        if let Some(severities) = &query.severities {
            if !severities.contains(&event.severity) {
                return false;
            }
        }

        // Statuses
        if let Some(statuses) = &query.statuses {
            if !statuses.contains(&event.status) {
                return false;
            }
        }

        // User IDs
        if let Some(user_ids) = &query.user_ids {
            if let Some(user_id) = event.user_id {
                if !user_ids.contains(&user_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Usernames
        if let Some(usernames) = &query.usernames {
            if let Some(username) = &event.username {
                if !usernames.contains(username) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Resources
        if let Some(resources) = &query.resources {
            if !resources.contains(&event.resource) {
                return false;
            }
        }

        // Actions
        if let Some(actions) = &query.actions {
            if !actions.contains(&event.action) {
                return false;
            }
        }

        // IP addresses
        if let Some(ip_addresses) = &query.ip_addresses {
            if let Some(ip) = &event.ip_address {
                if !ip_addresses.contains(ip) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Session IDs
        if let Some(session_ids) = &query.session_ids {
            if let Some(session_id) = event.session_id {
                if !session_ids.contains(&session_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Correlation IDs
        if let Some(correlation_ids) = &query.correlation_ids {
            if let Some(correlation_id) = event.correlation_id {
                if !correlation_ids.contains(&correlation_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Compliance tags
        if let Some(compliance_tags) = &query.compliance_tags {
            let has_matching_tag = compliance_tags.iter().any(|tag| event.compliance_tags.contains(tag));
            if !has_matching_tag {
                return false;
            }
        }

        true
    }

    /// Sort events by field
    fn sort_events(&self, events: &mut Vec<AuditEvent>, field: &str, direction: &SortDirection) {
        events.sort_by(|a, b| {
            let comparison = match field {
                "timestamp" => a.timestamp.cmp(&b.timestamp),
                "event_type" => a.event_type.to_string().cmp(&b.event_type.to_string()),
                "severity" => a.severity.to_string().cmp(&b.severity.to_string()),
                "username" => a.username.as_ref().unwrap_or(&"".to_string()).cmp(b.username.as_ref().unwrap_or(&"".to_string())),
                "resource" => a.resource.cmp(&b.resource),
                "action" => a.action.cmp(&b.action),
                _ => a.timestamp.cmp(&b.timestamp), // Default to timestamp
            };

            match direction {
                SortDirection::Asc => comparison,
                SortDirection::Desc => comparison.reverse(),
            }
        });
    }

    /// Count recent events
    async fn count_recent_events(&self, event_type: Option<AuditEventType>, status: Option<AuditStatus>, seconds_ago: i64) -> u32 {
        let events = self.events.read().await;
        let cutoff = Utc::now() - chrono::Duration::seconds(seconds_ago);

        events
            .iter()
            .filter(|event| {
                let time_match = event.timestamp > cutoff;
                let type_match = event_type.as_ref().map_or(true, |t| &event.event_type == t);
                let status_match = status.as_ref().map_or(true, |s| &event.status == s);
                time_match && type_match && status_match
            })
            .count() as u32
    }

    /// Check if event has field
    fn has_field(&self, event: &AuditEvent, field: &str) -> bool {
        match field {
            "user_id" => event.user_id.is_some(),
            "username" => event.username.is_some(),
            "resource" => !event.resource.is_empty(),
            "action" => !event.action.is_empty(),
            "ip_address" => event.ip_address.is_some(),
            "user_agent" => event.user_agent.is_some(),
            _ => false,
        }
    }
}

impl ComplianceChecker {
    pub fn new() -> Self {
        Self {
            requirements: vec![
                ComplianceRequirement {
                    name: "SOX Compliance".to_string(),
                    description: "Sarbanes-Oxley compliance requirements".to_string(),
                    required_fields: vec!["user_id".to_string(), "resource".to_string(), "action".to_string()],
                    retention_days: 2555, // 7 years
                    event_types: vec![
                        AuditEventType::Authentication,
                        AuditEventType::Authorization,
                        AuditEventType::DataAccess,
                        AuditEventType::AdminAction,
                    ],
                    alert_on_violation: true,
                },
                ComplianceRequirement {
                    name: "GDPR Compliance".to_string(),
                    description: "General Data Protection Regulation compliance".to_string(),
                    required_fields: vec!["user_id".to_string(), "resource".to_string(), "action".to_string(), "ip_address".to_string()],
                    retention_days: 2555, // 7 years
                    event_types: vec![
                        AuditEventType::DataAccess,
                        AuditEventType::Authentication,
                        AuditEventType::Authorization,
                    ],
                    alert_on_violation: true,
                },
                ComplianceRequirement {
                    name: "PCI DSS Compliance".to_string(),
                    description: "Payment Card Industry Data Security Standard".to_string(),
                    required_fields: vec!["user_id".to_string(), "resource".to_string(), "action".to_string(), "ip_address".to_string()],
                    retention_days: 365, // 1 year
                    event_types: vec![
                        AuditEventType::Authentication,
                        AuditEventType::Authorization,
                        AuditEventType::DataAccess,
                        AuditEventType::SystemOperation,
                    ],
                    alert_on_violation: true,
                },
            ],
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_policy: RetentionPolicy::default(),
            enable_real_time_alerts: true,
            alert_thresholds: AlertThresholds::default(),
            required_fields: vec!["user_id".to_string(), "resource".to_string(), "action".to_string()],
            sensitive_fields: vec!["password".to_string(), "token".to_string(), "secret".to_string()],
            compliance_requirements: vec![],
        }
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        let mut event_type_retention = HashMap::new();
        event_type_retention.insert(AuditEventType::Authentication, 90);
        event_type_retention.insert(AuditEventType::Authorization, 90);
        event_type_retention.insert(AuditEventType::AdminAction, 2555); // 7 years for admin actions
        event_type_retention.insert(AuditEventType::DataAccess, 2555); // 7 years for data access
        event_type_retention.insert(AuditEventType::Compliance, 2555); // 7 years for compliance

        let mut severity_retention = HashMap::new();
        severity_retention.insert(AuditSeverity::Critical, 2555); // 7 years for critical events
        severity_retention.insert(AuditSeverity::Error, 365); // 1 year for errors

        Self {
            default_retention_days: 180, // 6 months default
            event_type_retention,
            severity_retention,
            max_storage_size_gb: Some(100),
            enable_compression: true,
            enable_archival: true,
            archival_threshold_days: 90,
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            failed_authentications_per_minute: 5,
            failed_authorizations_per_minute: 10,
            critical_events_per_hour: 5,
            policy_violations_per_hour: 10,
            unusual_activity_score: 0.8,
        }
    }
}