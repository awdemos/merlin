//! Integration tests for security policies system

use merlin::security::policies::{
    PolicyEngine, SecurityPolicy, PolicyType, EnforcementLevel, PolicyStatus, PolicyRule,
    PolicyOperator, PolicySeverity, PolicyResult, PolicyEvaluation, PolicyViolation
};
use merlin::models::container_config::DockerContainerConfig;
use merlin::models::security_scan_config::ComplianceStandard;
use std::collections::HashMap;

#[tokio::test]
async fn test_policy_engine_creation() {
    let engine = PolicyEngine::new();

    // Initialize default templates
    engine.initialize_default_templates().await.expect("Failed to initialize default templates");

    // Verify engine was created successfully
    let templates = engine.policy_templates.read().await;
    assert!(templates.len() >= 2); // Should have at least container-security and resource-limits templates
}

#[tokio::test]
async fn test_security_policy_creation() {
    let engine = PolicyEngine::new();

    let policy = SecurityPolicy {
        id: uuid::Uuid::new_v4(),
        name: "test-policy".to_string(),
        description: "Test security policy".to_string(),
        policy_type: PolicyType::ContainerSecurity,
        version: "1.0".to_string(),
        enforcement_level: EnforcementLevel::Mandatory,
        status: PolicyStatus::Active,
        rules: vec![
            PolicyRule {
                id: uuid::Uuid::new_v4(),
                name: "non-root-user".to_string(),
                description: "Containers must run as non-root user".to_string(),
                field: "user".to_string(),
                operator: PolicyOperator::NotEquals,
                value: serde_json::Value::String("root".to_string()),
                severity: PolicySeverity::Critical,
                weight: 1.0,
            },
        ],
        conditions: vec![],
        actions: vec![],
        compliance_standards: vec![ComplianceStandard::CISDockerBenchmark],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        created_by: "test".to_string(),
        tags: HashMap::new(),
    };

    engine.create_policy(policy).await.expect("Failed to create policy");

    // Verify policy was created
    let policies = engine.get_policies().await;
    assert!(policies.len() >= 1);
    assert!(policies.iter().any(|p| p.name == "test-policy"));
}

#[tokio::test]
async fn test_policy_evaluation_secure_config() {
    let engine = PolicyEngine::new();
    engine.initialize_default_templates().await.expect("Failed to initialize default templates");

    // Create a secure container configuration
    let config = DockerContainerConfig::builder()
        .image_name("nginx:latest".to_string())
        .dockerfile_path("Dockerfile".to_string())
        .user(Some("1000".to_string()))
        .privileged(Some(false))
        .read_only(Some(true))
        .build()
        .expect("Failed to create container config");

    // Evaluate configuration
    let result = engine.evaluate_configuration(&config).await
        .expect("Failed to evaluate configuration");

    // Should pass evaluation
    assert_eq!(result.result, PolicyResult::Pass);
    assert!(result.violations.is_empty());
    assert!(result.score >= 80.0); // Should have high score for secure config
}

#[tokio::test]
async fn test_policy_evaluation_insecure_config() {
    let engine = PolicyEngine::new();
    engine.initialize_default_templates().await.expect("Failed to initialize default templates");

    // Create an insecure container configuration
    let config = DockerContainerConfig::builder()
        .image_name("nginx:latest".to_string())
        .dockerfile_path("Dockerfile".to_string())
        .user(Some("root".to_string()))
        .privileged(Some(true))
        .read_only(Some(false))
        .build()
        .expect("Failed to create container config");

    // Evaluate configuration
    let result = engine.evaluate_configuration(&config).await
        .expect("Failed to evaluate configuration");

    // Should fail evaluation with violations
    assert_eq!(result.result, PolicyResult::Fail);
    assert!(result.violations.len() >= 2); // Should have multiple violations
    assert!(result.score < 50.0); // Should have low score for insecure config

    // Check for specific violations
    let violation_messages: Vec<String> = result.violations.iter().map(|v| v.message.clone()).collect();
    assert!(violation_messages.iter().any(|m| m.contains("user")));
    assert!(violation_messages.iter().any(|m| m.contains("privileged")));
}

#[tokio::test]
async fn test_policy_types() {
    let engine = PolicyEngine::new();

    let policy_types = vec![
        PolicyType::ContainerSecurity,
        PolicyType::ImageScanning,
        PolicyType::NetworkPolicy,
        PolicyType::ResourcePolicy,
        PolicyType::AccessControl,
        PolicyType::Compliance,
        PolicyType::Deployment,
        PolicyType::Custom("custom-type".to_string()),
    ];

    for policy_type in policy_types {
        let policy = SecurityPolicy {
            id: uuid::Uuid::new_v4(),
            name: format!("test-policy-{:?}", policy_type),
            description: "Test policy".to_string(),
            policy_type,
            version: "1.0".to_string(),
            enforcement_level: EnforcementLevel::Recommended,
            status: PolicyStatus::Active,
            rules: vec![],
            conditions: vec![],
            actions: vec![],
            compliance_standards: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            created_by: "test".to_string(),
            tags: HashMap::new(),
        };

        engine.create_policy(policy).await.expect("Failed to create policy");
    }

    // Verify all policy types were created
    let policies = engine.get_policies().await;
    assert!(policies.len() >= 8);
}

#[tokio::test]
async fn test_enforcement_levels() {
    let engine = PolicyEngine::new();

    let enforcement_levels = vec![
        EnforcementLevel::Advisory,
        EnforcementLevel::Recommended,
        EnforcementLevel::Mandatory,
        EnforcementLevel::Critical,
    ];

    for level in enforcement_levels {
        let policy = SecurityPolicy {
            id: uuid::Uuid::new_v4(),
            name: format!("test-policy-{:?}", level),
            description: "Test policy".to_string(),
            policy_type: PolicyType::ContainerSecurity,
            version: "1.0".to_string(),
            enforcement_level: level,
            status: PolicyStatus::Active,
            rules: vec![],
            conditions: vec![],
            actions: vec![],
            compliance_standards: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            created_by: "test".to_string(),
            tags: HashMap::new(),
        };

        engine.create_policy(policy).await.expect("Failed to create policy");
    }

    // Verify all enforcement levels were created
    let policies = engine.get_policies().await;
    assert!(policies.len() >= 4);
}

#[tokio::test]
async fn test_policy_status_transitions() {
    let engine = PolicyEngine::new();

    let policy = SecurityPolicy {
        id: uuid::Uuid::new_v4(),
        name: "test-policy".to_string(),
        description: "Test security policy".to_string(),
        policy_type: PolicyType::ContainerSecurity,
        version: "1.0".to_string(),
        enforcement_level: EnforcementLevel::Mandatory,
        status: PolicyStatus::Active,
        rules: vec![],
        conditions: vec![],
        actions: vec![],
        compliance_standards: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        created_by: "test".to_string(),
        tags: HashMap::new(),
    };

    engine.create_policy(policy.clone()).await.expect("Failed to create policy");

    // Test status transitions
    let policy_id = policy.id;

    // Deactivate policy
    engine.update_policy_status(policy_id, PolicyStatus::Inactive).await
        .expect("Failed to update policy status");

    let retrieved_policy = engine.get_policy(policy_id).await
        .expect("Failed to retrieve policy");
    assert_eq!(retrieved_policy.status, PolicyStatus::Inactive);

    // Reactivate policy
    engine.update_policy_status(policy_id, PolicyStatus::Active).await
        .expect("Failed to update policy status");

    let retrieved_policy = engine.get_policy(policy_id).await
        .expect("Failed to retrieve policy");
    assert_eq!(retrieved_policy.status, PolicyStatus::Active);
}

#[tokio::test]
async fn test_policy_operators() {
    let engine = PolicyEngine::new();

    let operators = vec![
        PolicyOperator::Equals,
        PolicyOperator::NotEquals,
        PolicyOperator::Contains,
        PolicyOperator::NotContains,
        PolicyOperator::StartsWith,
        PolicyOperator::EndsWith,
        PolicyOperator::GreaterThan,
        PolicyOperator::LessThan,
        PolicyOperator::GreaterThanOrEqual,
        PolicyOperator::LessThanOrEqual,
        PolicyOperator::In,
        PolicyOperator::NotIn,
        PolicyOperator::Regex,
        PolicyOperator::Exists,
        PolicyOperator::NotExists,
    ];

    for operator in operators {
        let policy = SecurityPolicy {
            id: uuid::Uuid::new_v4(),
            name: format!("test-policy-{:?}", operator),
            description: "Test policy".to_string(),
            policy_type: PolicyType::ContainerSecurity,
            version: "1.0".to_string(),
            enforcement_level: EnforcementLevel::Recommended,
            status: PolicyStatus::Active,
            rules: vec![PolicyRule {
                id: uuid::Uuid::new_v4(),
                name: "test-rule".to_string(),
                description: "Test rule".to_string(),
                field: "test".to_string(),
                operator: operator.clone(),
                value: serde_json::Value::String("test".to_string()),
                severity: PolicySeverity::Medium,
                weight: 1.0,
            }],
            conditions: vec![],
            actions: vec![],
            compliance_standards: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            created_by: "test".to_string(),
            tags: HashMap::new(),
        };

        engine.create_policy(policy).await.expect("Failed to create policy");
    }

    // Verify all operators were used
    let policies = engine.get_policies().await;
    assert!(policies.len() >= 15);
}

#[tokio::test]
async fn test_policy_severity_levels() {
    let engine = PolicyEngine::new();

    let severities = vec![
        PolicySeverity::Low,
        PolicySeverity::Medium,
        PolicySeverity::High,
        PolicySeverity::Critical,
    ];

    for severity in severities {
        let policy = SecurityPolicy {
            id: uuid::Uuid::new_v4(),
            name: format!("test-policy-{:?}", severity),
            description: "Test policy".to_string(),
            policy_type: PolicyType::ContainerSecurity,
            version: "1.0".to_string(),
            enforcement_level: EnforcementLevel::Recommended,
            status: PolicyStatus::Active,
            rules: vec![PolicyRule {
                id: uuid::Uuid::new_v4(),
                name: "test-rule".to_string(),
                description: "Test rule".to_string(),
                field: "test".to_string(),
                operator: PolicyOperator::Equals,
                value: serde_json::Value::String("test".to_string()),
                severity: severity.clone(),
                weight: 1.0,
            }],
            conditions: vec![],
            actions: vec![],
            compliance_standards: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            created_by: "test".to_string(),
            tags: HashMap::new(),
        };

        engine.create_policy(policy).await.expect("Failed to create policy");
    }

    // Verify all severity levels were used
    let policies = engine.get_policies().await;
    assert!(policies.len() >= 4);
}

#[tokio::test]
async fn test_policy_filtering() {
    let engine = PolicyEngine::new();

    // Create policies of different types
    let container_policy = SecurityPolicy {
        id: uuid::Uuid::new_v4(),
        name: "container-security-policy".to_string(),
        description: "Container security policy".to_string(),
        policy_type: PolicyType::ContainerSecurity,
        version: "1.0".to_string(),
        enforcement_level: EnforcementLevel::Mandatory,
        status: PolicyStatus::Active,
        rules: vec![],
        conditions: vec![],
        actions: vec![],
        compliance_standards: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        created_by: "test".to_string(),
        tags: HashMap::new(),
    };

    let resource_policy = SecurityPolicy {
        id: uuid::Uuid::new_v4(),
        name: "resource-policy".to_string(),
        description: "Resource policy".to_string(),
        policy_type: PolicyType::ResourcePolicy,
        version: "1.0".to_string(),
        enforcement_level: EnforcementLevel::Recommended,
        status: PolicyStatus::Inactive,
        rules: vec![],
        conditions: vec![],
        actions: vec![],
        compliance_standards: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        created_by: "test".to_string(),
        tags: HashMap::new(),
    };

    engine.create_policy(container_policy).await.expect("Failed to create policy");
    engine.create_policy(resource_policy).await.expect("Failed to create policy");

    // Test filtering by type
    let container_policies = engine.get_policies_by_type(PolicyType::ContainerSecurity).await;
    assert_eq!(container_policies.len(), 1);
    assert_eq!(container_policies[0].name, "container-security-policy");

    // Test filtering by status
    let active_policies = engine.get_active_policies().await;
    assert_eq!(active_policies.len(), 1);
    assert_eq!(active_policies[0].name, "container-security-policy");
}

#[tokio::test]
async fn test_evaluation_history() {
    let engine = PolicyEngine::new();
    engine.initialize_default_templates().await.expect("Failed to initialize default templates");

    // Create a configuration and evaluate it multiple times
    let config = DockerContainerConfig::builder()
        .image_name("nginx:latest".to_string())
        .dockerfile_path("Dockerfile".to_string())
        .user(Some("1000".to_string()))
        .build()
        .expect("Failed to create container config");

    // Evaluate configuration
    engine.evaluate_configuration(&config).await.expect("Failed to evaluate configuration");
    engine.evaluate_configuration(&config).await.expect("Failed to evaluate configuration");

    // Check evaluation history
    let history = engine.get_evaluation_history(None).await;
    assert!(history.len() >= 2);

    // Check limited history
    let limited_history = engine.get_evaluation_history(Some(1)).await;
    assert_eq!(limited_history.len(), 1);
}

#[tokio::test]
async fn test_policy_from_template() {
    let engine = PolicyEngine::new();
    engine.initialize_default_templates().await.expect("Failed to initialize default templates");

    // Create policy from template
    let parameters = HashMap::new();
    let policy = engine.create_policy_from_template("container-security", parameters).await
        .expect("Failed to create policy from template");

    // Verify policy was created from template
    assert!(policy.name.contains("container-security"));
    assert_eq!(policy.policy_type, PolicyType::ContainerSecurity);
    assert_eq!(policy.enforcement_level, EnforcementLevel::Mandatory);
    assert_eq!(policy.rules.len(), 2); // Template should have 2 rules
}