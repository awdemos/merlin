//! Security policies module for Merlin AI Router Docker deployment
//! Provides policy management, enforcement, and compliance checking

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::models::container_config::DockerContainerConfig;
use crate::models::security_scan_config::{SecurityScanConfig, ComplianceStandard};
use super::hardening::SecurityControl;
use super::docker_client::DockerConfigError;

/// Security policy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyType {
    ContainerSecurity,
    ImageScanning,
    NetworkPolicy,
    ResourcePolicy,
    AccessControl,
    Compliance,
    Deployment,
    Custom(String),
}

/// Policy enforcement levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnforcementLevel {
    Advisory,      // Warning only, no enforcement
    Recommended,   // Strongly recommended but optional
    Mandatory,     // Must be enforced
    Critical,      // Critical for security operations
}

/// Policy status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyStatus {
    Active,
    Inactive,
    Deprecated,
    Draft,
    Disabled,
}

/// Security policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub policy_type: PolicyType,
    pub version: String,
    pub enforcement_level: EnforcementLevel,
    pub status: PolicyStatus,
    pub rules: Vec<PolicyRule>,
    pub conditions: Vec<PolicyCondition>,
    pub actions: Vec<PolicyAction>,
    pub compliance_standards: Vec<ComplianceStandard>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub tags: HashMap<String, String>,
}

/// Policy rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub field: String,
    pub operator: PolicyOperator,
    pub value: serde_json::Value,
    pub severity: PolicySeverity,
    pub weight: f64,
}

/// Policy operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    In,
    NotIn,
    Regex,
    Exists,
    NotExists,
}

/// Policy severity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Policy condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    pub id: Uuid,
    pub field: String,
    pub operator: PolicyOperator,
    pub value: serde_json::Value,
    pub logic: ConditionLogic,
}

/// Condition logic operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionLogic {
    And,
    Or,
    Not,
}

/// Policy action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyAction {
    pub id: Uuid,
    pub action_type: PolicyActionType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub conditions: Vec<PolicyCondition>,
}

/// Policy action types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyActionType {
    Block,
    Warn,
    Log,
    Notify,
    Quarantine,
    Remediate,
    Escalate,
    Custom(String),
}

/// Policy evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub config_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub result: PolicyResult,
    pub violations: Vec<PolicyViolation>,
    pub score: f64,
    pub recommendations: Vec<String>,
    pub enforcement_actions: Vec<PolicyAction>,
}

/// Policy evaluation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyResult {
    Pass,
    Fail,
    Warning,
    Error,
}

/// Policy violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub rule_name: String,
    pub field: String,
    pub actual_value: serde_json::Value,
    pub expected_value: serde_json::Value,
    pub severity: PolicySeverity,
    pub message: String,
}

/// Policy enforcement engine
#[derive(Clone)]
pub struct PolicyEngine {
    policies: Arc<RwLock<HashMap<Uuid, SecurityPolicy>>>,
    evaluation_history: Arc<RwLock<Vec<PolicyEvaluation>>>,
    policy_templates: Arc<RwLock<HashMap<String, SecurityPolicy>>>,
    compliance_framework: ComplianceFramework,
}

/// Compliance framework for policies
#[derive(Debug, Clone)]
pub struct ComplianceFramework {
    standards: HashMap<ComplianceStandard, Vec<SecurityPolicy>>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            evaluation_history: Arc::new(RwLock::new(Vec::new())),
            policy_templates: Arc::new(RwLock::new(HashMap::new())),
            compliance_framework: ComplianceFramework::new(),
        }
    }

    /// Create a new security policy
    pub async fn create_policy(&self, policy: SecurityPolicy) -> Result<(), DockerConfigError> {
        let mut policies = self.policies.write().await;
        policies.insert(policy.id, policy);
        Ok(())
    }

    /// Evaluate a container configuration against all applicable policies
    pub async fn evaluate_configuration(
        &self,
        config: &DockerContainerConfig,
    ) -> Result<PolicyEvaluation, DockerConfigError> {
        let policies = self.policies.read().await;
        let mut violations = Vec::new();
        let mut recommendations = Vec::new();
        let mut enforcement_actions = Vec::new();
        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        // Evaluate against all active policies
        for policy in policies.values() {
            if policy.status == PolicyStatus::Active {
                let policy_result = self.evaluate_policy(policy, config).await;

                violations.extend(policy_result.violations);
                recommendations.extend(policy_result.recommendations);
                enforcement_actions.extend(policy_result.enforcement_actions);

                total_score += policy_result.score;
                total_weight += 1.0;
            }
        }

        let overall_score = if total_weight > 0.0 {
            total_score / total_weight
        } else {
            100.0
        };

        let evaluation_result = if violations.is_empty() {
            PolicyResult::Pass
        } else {
            let critical_violations: Vec<_> = violations.iter()
                .filter(|v| v.severity == PolicySeverity::Critical)
                .collect();

            if !critical_violations.is_empty() {
                PolicyResult::Fail
            } else {
                PolicyResult::Warning
            }
        };

        let evaluation = PolicyEvaluation {
            id: Uuid::new_v4(),
            policy_id: Uuid::new_v4(), // This would be the policy set ID
            config_id: config.id,
            timestamp: Utc::now(),
            result: evaluation_result,
            violations,
            score: overall_score,
            recommendations,
            enforcement_actions,
        };

        // Store evaluation result
        let mut history = self.evaluation_history.write().await;
        history.push(evaluation.clone());

        // Keep only last 1000 evaluations
        if history.len() > 1000 {
            *history = history[history.len() - 1000..].to_vec();
        }

        Ok(evaluation)
    }

    /// Evaluate a single policy against a configuration
    async fn evaluate_policy(
        &self,
        policy: &SecurityPolicy,
        config: &DockerContainerConfig,
    ) -> PolicyEvaluation {
        let mut violations = Vec::new();
        let mut recommendations = Vec::new();
        let mut enforcement_actions = Vec::new();
        let mut score = 100.0;
        let mut total_weight = 0.0;

        // Evaluate policy conditions
        let conditions_met = self.evaluate_conditions(&policy.conditions, config).await;

        if conditions_met {
            // Evaluate policy rules
            for rule in &policy.rules {
                total_weight += rule.weight;

                if !self.evaluate_rule(rule, config).await {
                    let violation = PolicyViolation {
                        id: Uuid::new_v4(),
                        rule_id: rule.id,
                        rule_name: rule.name.clone(),
                        field: rule.field.clone(),
                        actual_value: self.get_field_value(config, &rule.field),
                        expected_value: rule.value.clone(),
                        severity: rule.severity.clone(),
                        message: format!("Policy rule '{}' violated: {} {}", rule.name, rule.field, self.operator_to_string(&rule.operator)),
                    };

                    violations.push(violation);

                    // Apply penalty based on severity
                    let penalty = match rule.severity {
                        PolicySeverity::Low => 5.0,
                        PolicySeverity::Medium => 15.0,
                        PolicySeverity::High => 30.0,
                        PolicySeverity::Critical => 50.0,
                    };

                    score -= penalty * rule.weight;

                    // Add recommendations
                    recommendations.push(format!("Fix policy violation: {}", rule.description));

                    // Determine enforcement actions
                    if policy.enforcement_level == EnforcementLevel::Mandatory ||
                       policy.enforcement_level == EnforcementLevel::Critical {
                        enforcement_actions.extend(policy.actions.clone());
                    }
                }
            }
        }

        // Normalize score
        if total_weight > 0.0 {
            score = (score / total_weight).max(0.0);
        }

        PolicyEvaluation {
            id: Uuid::new_v4(),
            policy_id: policy.id,
            config_id: config.id,
            timestamp: Utc::now(),
            result: if violations.is_empty() { PolicyResult::Pass } else { PolicyResult::Fail },
            violations,
            score,
            recommendations,
            enforcement_actions,
        }
    }

    /// Evaluate policy conditions
    async fn evaluate_conditions(
        &self,
        conditions: &[PolicyCondition],
        config: &DockerContainerConfig,
    ) -> bool {
        if conditions.is_empty() {
            return true;
        }

        // Simple condition evaluation (AND logic)
        for condition in conditions {
            if !self.evaluate_condition(condition, config).await {
                return false;
            }
        }

        true
    }

    /// Evaluate a single condition
    async fn evaluate_condition(
        &self,
        condition: &PolicyCondition,
        config: &DockerContainerConfig,
    ) -> bool {
        let field_value = self.get_field_value(config, &condition.field);
        self.evaluate_operator(&condition.operator, &field_value, &condition.value)
    }

    /// Evaluate a policy rule
    async fn evaluate_rule(&self, rule: &PolicyRule, config: &DockerContainerConfig) -> bool {
        let field_value = self.get_field_value(config, &rule.field);
        self.evaluate_operator(&rule.operator, &field_value, &rule.value)
    }

    /// Evaluate policy operator
    fn evaluate_operator(
        &self,
        operator: &PolicyOperator,
        field_value: &serde_json::Value,
        expected_value: &serde_json::Value,
    ) -> bool {
        match operator {
            PolicyOperator::Equals => field_value == expected_value,
            PolicyOperator::NotEquals => field_value != expected_value,
            PolicyOperator::Contains => {
                if let (Some(field_str), Some(expected_str)) = (
                    field_value.as_str(),
                    expected_value.as_str(),
                ) {
                    field_str.contains(expected_str)
                } else {
                    false
                }
            }
            PolicyOperator::GreaterThan => {
                if let (Some(field_num), Some(expected_num)) = (
                    field_value.as_f64(),
                    expected_value.as_f64(),
                ) {
                    field_num > expected_num
                } else {
                    false
                }
            }
            PolicyOperator::LessThan => {
                if let (Some(field_num), Some(expected_num)) = (
                    field_value.as_f64(),
                    expected_value.as_f64(),
                ) {
                    field_num < expected_num
                } else {
                    false
                }
            }
            PolicyOperator::Exists => !field_value.is_null(),
            PolicyOperator::NotExists => field_value.is_null(),
            _ => false, // Simplified for this example
        }
    }

    /// Get field value from configuration
    fn get_field_value(&self, config: &DockerContainerConfig, field: &str) -> serde_json::Value {
        match field {
            "image" => serde_json::Value::String(config.image_name.clone()),
            "privileged" => serde_json::Value::Bool(config.privileged.unwrap_or(false)),
            "read_only" => serde_json::Value::Bool(config.read_only.unwrap_or(false)),
            "user" => serde_json::Value::String(config.user.clone().unwrap_or_else(|| "root".to_string())),
            "memory_limit" => serde_json::Value::Number(serde_json::Number::from(config.host_config.memory.unwrap_or(0))),
            "cpu_shares" => serde_json::Value::Number(serde_json::Number::from(config.host_config.cpu_shares.unwrap_or(0))),
            _ => serde_json::Value::Null,
        }
    }

    /// Convert operator to string
    fn operator_to_string(&self, operator: &PolicyOperator) -> String {
        match operator {
            PolicyOperator::Equals => "==".to_string(),
            PolicyOperator::NotEquals => "!=".to_string(),
            PolicyOperator::Contains => "contains".to_string(),
            PolicyOperator::GreaterThan => ">".to_string(),
            PolicyOperator::LessThan => "<".to_string(),
            PolicyOperator::Exists => "exists".to_string(),
            PolicyOperator::NotExists => "not exists".to_string(),
            _ => format!("{:?}", operator),
        }
    }

    /// Get policy by ID
    pub async fn get_policy(&self, policy_id: Uuid) -> Option<SecurityPolicy> {
        let policies = self.policies.read().await;
        policies.get(&policy_id).cloned()
    }

    /// Get all policies
    pub async fn get_policies(&self) -> Vec<SecurityPolicy> {
        let policies = self.policies.read().await;
        policies.values().cloned().collect()
    }

    /// Get policies by type
    pub async fn get_policies_by_type(&self, policy_type: PolicyType) -> Vec<SecurityPolicy> {
        let policies = self.policies.read().await;
        policies
            .values()
            .filter(|p| p.policy_type == policy_type)
            .cloned()
            .collect()
    }

    /// Get active policies
    pub async fn get_active_policies(&self) -> Vec<SecurityPolicy> {
        let policies = self.policies.read().await;
        policies
            .values()
            .filter(|p| p.status == PolicyStatus::Active)
            .cloned()
            .collect()
    }

    /// Update policy status
    pub async fn update_policy_status(&self, policy_id: Uuid, status: PolicyStatus) -> Result<(), DockerConfigError> {
        let mut policies = self.policies.write().await;

        if let Some(policy) = policies.get_mut(&policy_id) {
            policy.status = status;
            policy.updated_at = Utc::now();
            Ok(())
        } else {
            Err(DockerConfigError::Validation("Policy not found".to_string()))
        }
    }

    /// Get evaluation history
    pub async fn get_evaluation_history(&self, limit: Option<usize>) -> Vec<PolicyEvaluation> {
        let history = self.evaluation_history.read().await;
        match limit {
            Some(limit) => history[history.len().saturating_sub(limit)..].to_vec(),
            None => history.clone(),
        }
    }

    /// Create policy from template
    pub async fn create_policy_from_template(
        &self,
        template_name: &str,
        parameters: HashMap<String, serde_json::Value>,
    ) -> Result<SecurityPolicy, DockerConfigError> {
        let templates = self.policy_templates.read().await;

        if let Some(template) = templates.get(template_name) {
            let mut policy = template.clone();
            policy.id = Uuid::new_v4();
            policy.name = format!("{}-{}", policy.name, Uuid::new_v4());
            policy.created_at = Utc::now();
            policy.updated_at = Utc::now();

            // Apply parameters to customize the policy
            self.apply_policy_parameters(&mut policy, parameters);

            Ok(policy)
        } else {
            Err(DockerConfigError::Validation("Template not found".to_string()))
        }
    }

    /// Apply parameters to policy
    fn apply_policy_parameters(&self, policy: &mut SecurityPolicy, parameters: HashMap<String, serde_json::Value>) {
        // Apply parameters to customize rules, conditions, etc.
        for (key, value) in parameters {
            if let Some(rule) = policy.rules.iter_mut().find(|r| r.name.contains(&key)) {
                rule.value = value.clone();
            }
        }
    }

    /// Initialize default policy templates
    pub async fn initialize_default_templates(&self) -> Result<(), DockerConfigError> {
        let templates = vec![
            SecurityPolicy {
                id: Uuid::new_v4(),
                name: "container-security".to_string(),
                description: "Container security baseline policy".to_string(),
                policy_type: PolicyType::ContainerSecurity,
                version: "1.0".to_string(),
                enforcement_level: EnforcementLevel::Mandatory,
                status: PolicyStatus::Active,
                rules: vec![
                    PolicyRule {
                        id: Uuid::new_v4(),
                        name: "non-root-user".to_string(),
                        description: "Containers must run as non-root user".to_string(),
                        field: "user".to_string(),
                        operator: PolicyOperator::NotEquals,
                        value: serde_json::Value::String("root".to_string()),
                        severity: PolicySeverity::Critical,
                        weight: 1.0,
                    },
                    PolicyRule {
                        id: Uuid::new_v4(),
                        name: "no-privileged".to_string(),
                        description: "Containers must not run in privileged mode".to_string(),
                        field: "privileged".to_string(),
                        operator: PolicyOperator::Equals,
                        value: serde_json::Value::Bool(false),
                        severity: PolicySeverity::Critical,
                        weight: 1.0,
                    },
                ],
                conditions: vec![],
                actions: vec![],
                compliance_standards: vec![ComplianceStandard::CISDockerBenchmark],
                created_at: Utc::now(),
                updated_at: Utc::now(),
                created_by: "system".to_string(),
                tags: {
                    let mut tags = HashMap::new();
                    tags.insert("category".to_string(), "security".to_string());
                    tags
                },
            },
            SecurityPolicy {
                id: Uuid::new_v4(),
                name: "resource-limits".to_string(),
                description: "Resource limits policy".to_string(),
                policy_type: PolicyType::ResourcePolicy,
                version: "1.0".to_string(),
                enforcement_level: EnforcementLevel::Recommended,
                status: PolicyStatus::Active,
                rules: vec![
                    PolicyRule {
                        id: Uuid::new_v4(),
                        name: "memory-limit".to_string(),
                        description: "Containers must have memory limits".to_string(),
                        field: "memory_limit".to_string(),
                        operator: PolicyOperator::GreaterThan,
                        value: serde_json::Value::Number(serde_json::Number::from(0)),
                        severity: PolicySeverity::Medium,
                        weight: 0.5,
                    },
                    PolicyRule {
                        id: Uuid::new_v4(),
                        name: "cpu-limit".to_string(),
                        description: "Containers must have CPU limits".to_string(),
                        field: "cpu_shares".to_string(),
                        operator: PolicyOperator::GreaterThan,
                        value: serde_json::Value::Number(serde_json::Number::from(0)),
                        severity: PolicySeverity::Medium,
                        weight: 0.5,
                    },
                ],
                conditions: vec![],
                actions: vec![],
                compliance_standards: vec![ComplianceStandard::CISDockerBenchmark],
                created_at: Utc::now(),
                updated_at: Utc::now(),
                created_by: "system".to_string(),
                tags: {
                    let mut tags = HashMap::new();
                    tags.insert("category".to_string(), "resources".to_string());
                    tags
                },
            },
        ];

        let mut policy_templates = self.policy_templates.write().await;
        for template in templates {
            policy_templates.insert(template.name.clone(), template);
        }

        Ok(())
    }
}

impl ComplianceFramework {
    pub fn new() -> Self {
        Self {
            standards: HashMap::new(),
        }
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "default-policy".to_string(),
            description: "Default security policy".to_string(),
            policy_type: PolicyType::ContainerSecurity,
            version: "1.0".to_string(),
            enforcement_level: EnforcementLevel::Recommended,
            status: PolicyStatus::Active,
            rules: vec![],
            conditions: vec![],
            actions: vec![],
            compliance_standards: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: "system".to_string(),
            tags: HashMap::new(),
        }
    }
}