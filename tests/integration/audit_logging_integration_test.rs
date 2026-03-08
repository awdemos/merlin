//! Integration tests for audit logging system

use merlin::security::audit_logging::{
    AuditService, AuditConfig, AuditEvent, AuditEventType, AuditSeverity, AuditStatus,
    AuditQuery, SortDirection, RetentionPolicy, AlertThresholds, ComplianceRequirement
};
use merlin::security::access_control::{User, Permission, UserRole};
use std::collections::HashMap;

#[tokio::test]
async fn test_audit_service_creation() {
    let config = AuditConfig::default();
    let service = AuditService::new(config);

    // Verify service was created successfully
    assert!(service.config.enabled);
    assert!(service.config.enable_real_time_alerts);
    assert_eq!(service.config.required_fields.len(), 3);
    assert_eq!(service.config.retention_policy.default_retention_days, 180);
}

#[tokio::test]
async fn test_audit_event_logging() {
    let config = AuditConfig::default();
    let service = AuditService::new(config);

    let event = AuditEvent {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        event_type: AuditEventType::Authentication,
        severity: AuditSeverity::Info,
        status: AuditStatus::Success,
        user_id: Some(uuid::Uuid::new_v4()),
        username: Some("testuser".to_string()),
        session_id: Some(uuid::Uuid::new_v4()),
        resource: "authentication".to_string(),
        action: "login".to_string(),
        details: {
            let mut details = HashMap::new();
            details.insert("method".to_string(), serde_json::Value::String("password".to_string()));
            details
        },
        ip_address: Some("192.168.1.1".to_string()),
        user_agent: Some("test-agent".to_string()),
        correlation_id: Some(uuid::Uuid::new_v4()),
        parent_event_id: None,
        duration_ms: Some(100),
        result: Some(serde_json::Value::Bool(true)),
        error_message: None,
        compliance_tags: vec!["authentication".to_string()],
    };

    service.log_event(event).await.expect("Failed to log audit event");

    // Verify event was logged
    let query = AuditQuery {
        limit: Some(1),
        ..Default::default()
    };
    let events = service.query_events(query).await.expect("Failed to query events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].username, Some("testuser".to_string()));
}

#[tokio::test]
async fn test_authentication_logging() {
    let config = AuditConfig::default();
    let service = AuditService::new(config);

    // Test successful authentication
    service.log_authentication(
        "testuser",
        Some(uuid::Uuid::new_v4()),
        true,
        Some("192.168.1.1".to_string()),
        Some("test-agent".to_string()),
        Some(uuid::Uuid::new_v4()),
        None,
    ).await.expect("Failed to log successful authentication");

    // Test failed authentication
    service.log_authentication(
        "testuser",
        None,
        false,
        Some("192.168.1.1".to_string()),
        Some("test-agent".to_string()),
        None,
        Some("Invalid credentials".to_string()),
    ).await.expect("Failed to log failed authentication");

    // Verify both events were logged
    let query = AuditQuery {
        event_types: Some(vec![AuditEventType::Authentication]),
        limit: Some(10),
        ..Default::default()
    };
    let events = service.query_events(query).await.expect("Failed to query events");
    assert_eq!(events.len(), 2);

    // Verify event details
    let success_event = events.iter().find(|e| e.status == AuditStatus::Success).unwrap();
    let failed_event = events.iter().find(|e| e.status == AuditStatus::Failure).unwrap();

    assert_eq!(success_event.severity, AuditSeverity::Info);
    assert_eq!(failed_event.severity, AuditSeverity::Warning);
    assert_eq!(failed_event.error_message, Some("Invalid credentials".to_string()));
}

#[tokio::test]
async fn test_authorization_logging() {
    let config = AuditConfig::default();
    let service = AuditService::new(config);

    let user = User {
        id: uuid::Uuid::new_v4(),
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        full_name: "Test User".to_string(),
        role: UserRole::Developer,
        permissions: vec![],
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_login: None,
        attributes: HashMap::new(),
    };

    // Test successful authorization
    service.log_authorization(
        &user,
        &Permission::ReadContainer,
        "container:nginx",
        true,
        Some("192.168.1.1".to_string()),
        Some("test-agent".to_string()),
        Some(uuid::Uuid::new_v4()),
        None,
    ).await.expect("Failed to log successful authorization");

    // Test failed authorization
    service.log_authorization(
        &user,
        &Permission::DeleteContainer,
        "container:nginx",
        false,
        Some("192.168.1.1".to_string()),
        Some("test-agent".to_string()),
        Some(uuid::Uuid::new_v4()),
        Some("Insufficient permissions".to_string()),
    ).await.expect("Failed to log failed authorization");

    // Verify both events were logged
    let query = AuditQuery {
        event_types: Some(vec![AuditEventType::Authorization]),
        limit: Some(10),
        ..Default::default()
    };
    let events = service.query_events(query).await.expect("Failed to query events");
    assert_eq!(events.len(), 2);

    // Verify event details
    let success_event = events.iter().find(|e| e.status == AuditStatus::Success).unwrap();
    let failed_event = events.iter().find(|e| e.status == AuditStatus::Blocked).unwrap();

    assert_eq!(success_event.severity, AuditSeverity::Info);
    assert_eq!(failed_event.severity, AuditSeverity::Warning);
    assert_eq!(failed_event.error_message, Some("Insufficient permissions".to_string()));
}

#[tokio::test]
async fn test_audit_event_querying() {
    let config = AuditConfig::default();
    let service = AuditService::new(config);

    // Log multiple events of different types
    service.log_authentication(
        "user1",
        Some(uuid::Uuid::new_v4()),
        true,
        Some("192.168.1.1".to_string()),
        None,
        None,
        None,
    ).await.expect("Failed to log authentication event");

    service.log_authorization(
        &User::default(),
        &Permission::ReadContainer,
        "container:test",
        true,
        Some("192.168.1.1".to_string()),
        None,
        None,
        None,
    ).await.expect("Failed to log authorization event");

    // Query by event type
    let auth_query = AuditQuery {
        event_types: Some(vec![AuditEventType::Authentication]),
        limit: Some(10),
        ..Default::default()
    };
    let auth_events = service.query_events(auth_query).await.expect("Failed to query auth events");
    assert_eq!(auth_events.len(), 1);
    assert_eq!(auth_events[0].event_type, AuditEventType::Authentication);

    // Query by username
    let user_query = AuditQuery {
        usernames: Some(vec!["user1".to_string()]),
        limit: Some(10),
        ..Default::default()
    };
    let user_events = service.query_events(user_query).await.expect("Failed to query user events");
    assert_eq!(user_events.len(), 1);
    assert_eq!(user_events[0].username, Some("user1".to_string()));

    // Query by IP address
    let ip_query = AuditQuery {
        ip_addresses: Some(vec!["192.168.1.1".to_string()]),
        limit: Some(10),
        ..Default::default()
    };
    let ip_events = service.query_events(ip_query).await.expect("Failed to query IP events");
    assert_eq!(ip_events.len(), 2); // Both events have the same IP
}

#[tokio::test]
async fn test_audit_statistics() {
    let config = AuditConfig::default();
    let service = AuditService::new(config);

    // Log multiple events
    for i in 0..5 {
        service.log_authentication(
            &format!("user{}", i),
            Some(uuid::Uuid::new_v4()),
            true,
            Some("192.168.1.1".to_string()),
            None,
            None,
            None,
        ).await.expect("Failed to log authentication event");
    }

    // Log some authorization events
    for i in 0..3 {
        service.log_authorization(
            &User::default(),
            &Permission::ReadContainer,
            &format!("container:test{}", i),
            true,
            Some("192.168.1.1".to_string()),
            None,
            None,
            None,
        ).await.expect("Failed to log authorization event");
    }

    // Get statistics
    let stats = service.get_statistics(None).await.expect("Failed to get statistics");

    assert_eq!(stats.total_events, 8);
    assert!(stats.events_by_type.contains_key(&AuditEventType::Authentication));
    assert!(stats.events_by_type.contains_key(&AuditEventType::Authorization));
    assert_eq!(*stats.events_by_type.get(&AuditEventType::Authentication).unwrap(), 5);
    assert_eq!(*stats.events_by_type.get(&AuditEventType::Authorization).unwrap(), 3);
}

#[tokio::test]
async fn test_audit_retention_policy() {
    let mut config = AuditConfig::default();
    config.retention_policy.default_retention_days = 1;
    let service = AuditService::new(config);

    // Log an event
    let old_timestamp = chrono::Utc::now() - chrono::Duration::days(2);
    let old_event = AuditEvent {
        id: uuid::Uuid::new_v4(),
        timestamp: old_timestamp,
        event_type: AuditEventType::Authentication,
        severity: AuditSeverity::Info,
        status: AuditStatus::Success,
        user_id: Some(uuid::Uuid::new_v4()),
        username: Some("olduser".to_string()),
        session_id: Some(uuid::Uuid::new_v4()),
        resource: "authentication".to_string(),
        action: "login".to_string(),
        details: HashMap::new(),
        ip_address: None,
        user_agent: None,
        correlation_id: None,
        parent_event_id: None,
        duration_ms: None,
        result: Some(serde_json::Value::Bool(true)),
        error_message: None,
        compliance_tags: vec![],
    };

    service.log_event(old_event).await.expect("Failed to log old event");

    // Log a recent event
    service.log_authentication(
        "newuser",
        Some(uuid::Uuid::new_v4()),
        true,
        None,
        None,
        None,
        None,
    ).await.expect("Failed to log recent event");

    // Query all events
    let query = AuditQuery {
        limit: Some(10),
        ..Default::default()
    };
    let events = service.query_events(query).await.expect("Failed to query events");

    // Only recent event should remain
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].username, Some("newuser".to_string()));
}

#[tokio::test]
async fn test_audit_alerts() {
    let mut config = AuditConfig::default();
    config.alert_thresholds.failed_authentications_per_minute = 2;
    let service = AuditService::new(config);

    // Log multiple failed authentications to trigger alert
    for _ in 0..3 {
        service.log_authentication(
            "testuser",
            None,
            false,
            Some("192.168.1.1".to_string()),
            None,
            None,
            Some("Invalid credentials".to_string()),
        ).await.expect("Failed to log failed authentication");
    }

    // Check for alerts
    let alerts = service.get_active_alerts().await;
    assert!(alerts.len() >= 1); // Should have at least one alert

    // Verify alert details
    let auth_alert = alerts.iter().find(|a| matches!(a.alert_type, merlin::security::audit_logging::AlertType::AuthenticationFailure));
    assert!(auth_alert.is_some());
    assert!(!auth_alert.unwrap().resolved);
}

#[tokio::test]
async fn test_audit_event_types() {
    let config = AuditConfig::default();
    let service = AuditService::new(config);

    let event_types = vec![
        AuditEventType::Authentication,
        AuditEventType::Authorization,
        AuditEventType::ConfigurationChange,
        AuditEventType::ContainerOperation,
        AuditEventType::SecurityScan,
        AuditEventType::PolicyViolation,
        AuditEventType::Deployment,
        AuditEventType::SystemOperation,
        AuditEventType::DataAccess,
        AuditEventType::AdminAction,
        AuditEventType::Compliance,
        AuditEventType::Custom("custom_event".to_string()),
    ];

    for event_type in event_types {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: event_type.clone(),
            severity: AuditSeverity::Info,
            status: AuditStatus::Success,
            user_id: Some(uuid::Uuid::new_v4()),
            username: Some("testuser".to_string()),
            session_id: Some(uuid::Uuid::new_v4()),
            resource: "test_resource".to_string(),
            action: "test_action".to_string(),
            details: HashMap::new(),
            ip_address: None,
            user_agent: None,
            correlation_id: None,
            parent_event_id: None,
            duration_ms: None,
            result: Some(serde_json::Value::Bool(true)),
            error_message: None,
            compliance_tags: vec![],
        };

        service.log_event(event).await.expect("Failed to log event");
    }

    // Verify all events were logged
    let query = AuditQuery {
        limit: Some(20),
        ..Default::default()
    };
    let events = service.query_events(query).await.expect("Failed to query events");
    assert!(events.len() >= 13);
}

#[tokio::test]
async fn test_audit_severity_levels() {
    let config = AuditConfig::default();
    let service = AuditService::new(config);

    let severities = vec![
        AuditSeverity::Debug,
        AuditSeverity::Info,
        AuditSeverity::Warning,
        AuditSeverity::Error,
        AuditSeverity::Critical,
    ];

    for severity in severities {
        service.log_authentication(
            &format!("user_{:?}", severity),
            Some(uuid::Uuid::new_v4()),
            true,
            None,
            None,
            None,
            None,
        ).await.expect("Failed to log authentication event");

        // Update the last event's severity
        let mut events = service.events.write().await;
        if let Some(event) = events.last_mut() {
            event.severity = severity.clone();
        }
    }

    // Query by severity
    for severity in severities {
        let query = AuditQuery {
            severities: Some(vec![severity.clone()]),
            limit: Some(1),
            ..Default::default()
        };
        let events = service.query_events(query).await.expect("Failed to query events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].severity, severity);
    }
}

#[tokio::test]
async fn test_audit_sorting() {
    let config = AuditConfig::default();
    let service = AuditService::new(config);

    // Log events with different timestamps
    let base_time = chrono::Utc::now();
    for i in 0..5 {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: base_time + chrono::Duration::seconds(i as i64),
            event_type: AuditEventType::Authentication,
            severity: AuditSeverity::Info,
            status: AuditStatus::Success,
            user_id: Some(uuid::Uuid::new_v4()),
            username: Some(format!("user{}", i)),
            session_id: Some(uuid::Uuid::new_v4()),
            resource: "authentication".to_string(),
            action: "login".to_string(),
            details: HashMap::new(),
            ip_address: None,
            user_agent: None,
            correlation_id: None,
            parent_event_id: None,
            duration_ms: None,
            result: Some(serde_json::Value::Bool(true)),
            error_message: None,
            compliance_tags: vec![],
        };

        service.log_event(event).await.expect("Failed to log event");
    }

    // Test ascending sort
    let query = AuditQuery {
        sort_field: Some("timestamp".to_string()),
        sort_direction: Some(SortDirection::Asc),
        limit: Some(5),
        ..Default::default()
    };
    let events = service.query_events(query).await.expect("Failed to query events");

    // Verify ascending order
    for i in 1..events.len() {
        assert!(events[i-1].timestamp <= events[i].timestamp);
    }

    // Test descending sort
    let query = AuditQuery {
        sort_field: Some("timestamp".to_string()),
        sort_direction: Some(SortDirection::Desc),
        limit: Some(5),
        ..Default::default()
    };
    let events = service.query_events(query).await.expect("Failed to query events");

    // Verify descending order
    for i in 1..events.len() {
        assert!(events[i-1].timestamp >= events[i].timestamp);
    }
}