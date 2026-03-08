//! Access control module for Merlin AI Router Docker deployment
//! Provides role-based access control (RBAC), authentication, and authorization

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::models::container_config::DockerContainerConfig;
use super::docker_client::DockerConfigError;

/// User roles for access control
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    Operator,
    Developer,
    Viewer,
    Auditor,
    Custom(String),
}

/// Permission types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    // Container operations
    CreateContainer,
    ReadContainer,
    UpdateContainer,
    DeleteContainer,
    StartContainer,
    StopContainer,
    RestartContainer,

    // Security operations
    RunSecurityScan,
    ViewSecurityResults,
    ConfigureSecurityPolicies,

    // Deployment operations
    DeployContainer,
    RollbackDeployment,
    ViewDeploymentHistory,

    // System operations
    ViewSystemMetrics,
    ViewLogs,
    ConfigureSystem,

    // Administrative operations
    ManageUsers,
    ManageRoles,
    ManagePolicies,
    SystemAdmin,

    // Custom permissions
    Custom(String),
}

/// User account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub role: UserRole,
    pub permissions: Vec<Permission>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Access control policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub resource: String,
    pub actions: Vec<Permission>,
    pub effect: PolicyEffect,
    pub conditions: Vec<AccessCondition>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Policy effect (allow or deny)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Access condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

/// Condition operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    In,
    NotIn,
    Regex,
}

/// Authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub scopes: Vec<String>,
}

/// Access request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRequest {
    pub user_id: Uuid,
    pub resource: String,
    pub action: Permission,
    pub context: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

/// Access decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessDecision {
    pub request_id: Uuid,
    pub allowed: bool,
    pub reason: String,
    pub policies_evaluated: Vec<PolicyEvaluationResult>,
    pub timestamp: DateTime<Utc>,
}

/// Policy evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationResult {
    pub policy_id: Uuid,
    pub policy_name: String,
    pub matched: bool,
    pub effect: PolicyEffect,
    pub conditions_met: bool,
}

/// Access control service
#[derive(Clone)]
pub struct AccessControlService {
    users: Arc<RwLock<HashMap<Uuid, User>>>,
    roles: Arc<RwLock<HashMap<Uuid, Role>>>,
    policies: Arc<RwLock<HashMap<Uuid, AccessPolicy>>>,
    tokens: Arc<RwLock<HashMap<String, AuthToken>>>,
    access_log: Arc<RwLock<Vec<AccessLogEntry>>>,
    session_manager: SessionManager,
}

/// Access log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub resource: String,
    pub action: Permission,
    pub allowed: bool,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub reason: String,
}

/// Session manager
#[derive(Debug, Clone)]
pub struct SessionManager {
    max_sessions_per_user: u32,
    session_timeout_hours: u64,
    active_sessions: Arc<RwLock<HashMap<Uuid, UserSession>>>,
}

/// User session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: String,
    pub is_active: bool,
}

impl AccessControlService {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            roles: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            access_log: Arc::new(RwLock::new(Vec::new())),
            session_manager: SessionManager::new(),
        }
    }

    /// Create a new user
    pub async fn create_user(&self, user: User) -> Result<(), DockerConfigError> {
        let mut users = self.users.write().await;
        users.insert(user.id, user);
        Ok(())
    }

    /// Create a new role
    pub async fn create_role(&self, role: Role) -> Result<(), DockerConfigError> {
        let mut roles = self.roles.write().await;
        roles.insert(role.id, role);
        Ok(())
    }

    /// Create a new access policy
    pub async fn create_policy(&self, policy: AccessPolicy) -> Result<(), DockerConfigError> {
        let mut policies = self.policies.write().await;
        policies.insert(policy.id, policy);
        Ok(())
    }

    /// Authenticate user with username and password
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<AuthToken, DockerConfigError> {
        let users = self.users.read().await;

        // Find user by username
        let user = users.values()
            .find(|u| u.username == username)
            .ok_or_else(|| DockerConfigError::Validation("User not found".to_string()))?;

        // Check if user is active
        if !user.is_active {
            return Err(DockerConfigError::Validation("User is not active".to_string()));
        }

        // In a real implementation, you would verify the password hash here
        // For this example, we'll accept any password
        if password.is_empty() {
            return Err(DockerConfigError::Validation("Invalid password".to_string()));
        }

        // Create authentication token
        let token = AuthToken {
            id: Uuid::new_v4(),
            user_id: user.id,
            token: self.generate_token(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            created_at: Utc::now(),
            last_used: None,
            is_active: true,
            scopes: self.get_user_scopes(user).await,
        };

        // Store token
        let mut tokens = self.tokens.write().await;
        tokens.insert(token.token.clone(), token.clone());

        // Update user's last login
        let mut users = self.users.write().await;
        if let Some(user) = users.get_mut(&user.id) {
            user.last_login = Some(Utc::now());
        }

        Ok(token)
    }

    /// Validate authentication token
    pub async fn validate_token(&self, token: &str) -> Result<User, DockerConfigError> {
        let tokens = self.tokens.read().await;

        let auth_token = tokens.get(token)
            .ok_or_else(|| DockerConfigError::Validation("Invalid token".to_string()))?;

        // Check if token is active and not expired
        if !auth_token.is_active {
            return Err(DockerConfigError::Validation("Token is not active".to_string()));
        }

        if auth_token.expires_at < Utc::now() {
            return Err(DockerConfigError::Validation("Token has expired".to_string()));
        }

        // Get user
        let users = self.users.read().await;
        let user = users.get(&auth_token.user_id)
            .ok_or_else(|| DockerConfigError::Validation("User not found".to_string()))?;

        if !user.is_active {
            return Err(DockerConfigError::Validation("User is not active".to_string()));
        }

        // Update last used time
        drop(tokens);
        let mut tokens = self.tokens.write().await;
        if let Some(token) = tokens.get_mut(token) {
            token.last_used = Some(Utc::now());
        }

        Ok(user.clone())
    }

    /// Check if user has permission
    pub async fn has_permission(&self, user_id: Uuid, permission: &Permission) -> bool {
        let users = self.users.read().await;
        let user = match users.get(&user_id) {
            Some(user) => user,
            None => return false,
        };

        // Check user's direct permissions
        if user.permissions.contains(permission) {
            return true;
        }

        // Check role-based permissions
        let roles = self.roles.read().await;
        if let Some(role) = roles.values().find(|r| r.name == format!("{:?}", user.role)) {
            if role.permissions.contains(permission) {
                return true;
            }
        }

        false
    }

    /// Evaluate access request
    pub async fn evaluate_access(&self, request: AccessRequest) -> Result<AccessDecision, DockerConfigError> {
        let mut policy_results = Vec::new();
        let mut final_decision = false;
        let mut reason = String::new();

        // Get user
        let users = self.users.read().await;
        let user = users.get(&request.user_id)
            .ok_or_else(|| DockerConfigError::Validation("User not found".to_string()))?;

        // Check if user is active
        if !user.is_active {
            return Ok(AccessDecision {
                request_id: request.user_id,
                allowed: false,
                reason: "User is not active".to_string(),
                policies_evaluated: policy_results,
                timestamp: Utc::now(),
            });
        }

        // Check direct permission
        if !self.has_permission(request.user_id, &request.action) {
            return Ok(AccessDecision {
                request_id: request.user_id,
                allowed: false,
                reason: "User does not have required permission".to_string(),
                policies_evaluated: policy_results,
                timestamp: Utc::now(),
            });
        }

        // Evaluate access policies
        let policies = self.policies.read().await;
        let mut highest_priority_deny = None;
        let mut highest_priority_allow = None;

        for policy in policies.values() {
            if policy.resource == request.resource {
                let evaluation = self.evaluate_policy(policy, &request).await;
                policy_results.push(evaluation.clone());

                if evaluation.matched && evaluation.conditions_met {
                    if evaluation.effect == PolicyEffect::Deny {
                        if highest_priority_deny.is_none() || policy.priority > highest_priority_deny.unwrap() {
                            highest_priority_deny = Some(policy.priority);
                        }
                    } else {
                        if highest_priority_allow.is_none() || policy.priority > highest_priority_allow.unwrap() {
                            highest_priority_allow = Some(policy.priority);
                        }
                    }
                }
            }
        }

        // Make decision based on policy priorities
        if let Some(deny_priority) = highest_priority_deny {
            if highest_priority_allow.is_none() || deny_priority > highest_priority_allow.unwrap() {
                final_decision = false;
                reason = "Access denied by policy".to_string();
            }
        } else if highest_priority_allow.is_some() {
            final_decision = true;
            reason = "Access allowed by policy".to_string();
        } else {
            final_decision = true;
            reason = "Access allowed by default".to_string();
        }

        let decision = AccessDecision {
            request_id: request.user_id,
            allowed: final_decision,
            reason: reason.clone(),
            policies_evaluated: policy_results,
            timestamp: Utc::now(),
        };

        // Log access decision
        self.log_access_decision(&request, &decision).await;

        Ok(decision)
    }

    /// Evaluate a single policy
    async fn evaluate_policy(&self, policy: &AccessPolicy, request: &AccessRequest) -> PolicyEvaluationResult {
        let action_matched = policy.actions.contains(&request.action);
        let conditions_met = self.evaluate_conditions(&policy.conditions, &request.context).await;

        PolicyEvaluationResult {
            policy_id: policy.id,
            policy_name: policy.name.clone(),
            matched: action_matched,
            effect: policy.effect.clone(),
            conditions_met,
        }
    }

    /// Evaluate policy conditions
    async fn evaluate_conditions(
        &self,
        conditions: &[AccessCondition],
        context: &HashMap<String, serde_json::Value>,
    ) -> bool {
        for condition in conditions {
            let context_value = context.get(&condition.field).cloned().unwrap_or(serde_json::Value::Null);

            let condition_met = match condition.operator {
                ConditionOperator::Equals => context_value == condition.value,
                ConditionOperator::NotEquals => context_value != condition.value,
                ConditionOperator::Contains => {
                    if let (Some(ctx_str), Some(val_str)) = (context_value.as_str(), condition.value.as_str()) {
                        ctx_str.contains(val_str)
                    } else {
                        false
                    }
                },
                ConditionOperator::GreaterThan => {
                    if let (Some(ctx_num), Some(val_num)) = (context_value.as_f64(), condition.value.as_f64()) {
                        ctx_num > val_num
                    } else {
                        false
                    }
                },
                ConditionOperator::LessThan => {
                    if let (Some(ctx_num), Some(val_num)) = (context_value.as_f64(), condition.value.as_f64()) {
                        ctx_num < val_num
                    } else {
                        false
                    }
                },
                _ => false, // Simplified for this example
            };

            if !condition_met {
                return false;
            }
        }

        true
    }

    /// Log access decision
    async fn log_access_decision(&self, request: &AccessRequest, decision: &AccessDecision) {
        let log_entry = AccessLogEntry {
            id: Uuid::new_v4(),
            user_id: request.user_id,
            resource: request.resource.clone(),
            action: request.action.clone(),
            allowed: decision.allowed,
            timestamp: Utc::now(),
            ip_address: request.context.get("ip_address").and_then(|v| v.as_str()).map(|s| s.to_string()),
            user_agent: request.context.get("user_agent").and_then(|v| v.as_str()).map(|s| s.to_string()),
            reason: decision.reason.clone(),
        };

        let mut access_log = self.access_log.write().await;
        access_log.push(log_entry);

        // Keep only last 10000 entries
        if access_log.len() > 10000 {
            *access_log = access_log[access_log.len() - 10000..].to_vec();
        }
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: Uuid) -> Option<User> {
        let users = self.users.read().await;
        users.get(&user_id).cloned()
    }

    /// Get user by username
    pub async fn get_user_by_username(&self, username: &str) -> Option<User> {
        let users = self.users.read().await;
        users.values().find(|u| u.username == username).cloned()
    }

    /// Get all users
    pub async fn get_users(&self) -> Vec<User> {
        let users = self.users.read().await;
        users.values().cloned().collect()
    }

    /// Get access log
    pub async fn get_access_log(&self, limit: Option<usize>) -> Vec<AccessLogEntry> {
        let access_log = self.access_log.read().await;
        match limit {
            Some(limit) => access_log[access_log.len().saturating_sub(limit)..].to_vec(),
            None => access_log.clone(),
        }
    }

    /// Generate authentication token
    fn generate_token(&self) -> String {
        format!("merlin_token_{}", Uuid::new_v4())
    }

    /// Get user scopes based on permissions
    async fn get_user_scopes(&self, user: &User) -> Vec<String> {
        let mut scopes = Vec::new();

        // Add role-based scopes
        scopes.push(format!("role:{}", user.username));

        // Add permission-based scopes
        for permission in &user.permissions {
            scopes.push(format!("perm:{}", self.permission_to_string(permission)));
        }

        // Add custom scopes based on user attributes
        if let Some(department) = user.attributes.get("department") {
            if let Some(dept_str) = department.as_str() {
                scopes.push(format!("dept:{}", dept_str));
            }
        }

        scopes
    }

    /// Convert permission to string
    fn permission_to_string(&self, permission: &Permission) -> String {
        match permission {
            Permission::CreateContainer => "create:container".to_string(),
            Permission::ReadContainer => "read:container".to_string(),
            Permission::UpdateContainer => "update:container".to_string(),
            Permission::DeleteContainer => "delete:container".to_string(),
            Permission::RunSecurityScan => "run:security_scan".to_string(),
            Permission::ViewSecurityResults => "view:security_results".to_string(),
            Permission::DeployContainer => "deploy:container".to_string(),
            Permission::ViewSystemMetrics => "view:system_metrics".to_string(),
            Permission::ManageUsers => "manage:users".to_string(),
            Permission::SystemAdmin => "admin:system".to_string(),
            Permission::Custom(name) => format!("custom:{}", name),
            _ => format!("{:?}", permission).to_lowercase(),
        }
    }

    /// Initialize default roles and policies
    pub async fn initialize_defaults(&self) -> Result<(), DockerConfigError> {
        // Create default roles
        let admin_role = Role {
            id: Uuid::new_v4(),
            name: "admin".to_string(),
            description: "System administrator with full access".to_string(),
            permissions: vec![
                Permission::SystemAdmin,
                Permission::ManageUsers,
                Permission::ManageRoles,
                Permission::ManagePolicies,
                Permission::CreateContainer,
                Permission::ReadContainer,
                Permission::UpdateContainer,
                Permission::DeleteContainer,
                Permission::RunSecurityScan,
                Permission::ViewSecurityResults,
                Permission::DeployContainer,
                Permission::ViewSystemMetrics,
            ],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let operator_role = Role {
            id: Uuid::new_v4(),
            name: "operator".to_string(),
            description: "System operator with operational access".to_string(),
            permissions: vec![
                Permission::CreateContainer,
                Permission::ReadContainer,
                Permission::UpdateContainer,
                Permission::StartContainer,
                Permission::StopContainer,
                Permission::RunSecurityScan,
                Permission::ViewSecurityResults,
                Permission::DeployContainer,
                Permission::ViewSystemMetrics,
                Permission::ViewLogs,
            ],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let viewer_role = Role {
            id: Uuid::new_v4(),
            name: "viewer".to_string(),
            description: "Viewer with read-only access".to_string(),
            permissions: vec![
                Permission::ReadContainer,
                Permission::ViewSecurityResults,
                Permission::ViewSystemMetrics,
                Permission::ViewLogs,
            ],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Create roles
        let mut roles = self.roles.write().await;
        roles.insert(admin_role.id, admin_role);
        roles.insert(operator_role.id, operator_role);
        roles.insert(viewer_role.id, viewer_role);

        // Create default admin user
        let admin_user = User {
            id: Uuid::new_v4(),
            username: "admin".to_string(),
            email: "admin@merlin.local".to_string(),
            full_name: "System Administrator".to_string(),
            role: UserRole::Admin,
            permissions: vec![],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("department".to_string(), serde_json::Value::String("IT".to_string()));
                attrs
            },
        };

        let mut users = self.users.write().await;
        users.insert(admin_user.id, admin_user);

        Ok(())
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            max_sessions_per_user: 5,
            session_timeout_hours: 24,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create new session
    pub async fn create_session(&self, user_id: Uuid, ip_address: String, user_agent: String) -> Result<UserSession, DockerConfigError> {
        let sessions = self.active_sessions.read().await;
        let user_sessions: Vec<_> = sessions.values()
            .filter(|s| s.user_id == user_id && s.is_active)
            .collect();

        // Check session limit
        if user_sessions.len() >= self.max_sessions_per_user as usize {
            return Err(DockerConfigError::Validation("Maximum sessions exceeded".to_string()));
        }

        let session = UserSession {
            id: Uuid::new_v4(),
            user_id,
            token: format!("session_{}", Uuid::new_v4()),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(self.session_timeout_hours as i64),
            last_activity: Utc::now(),
            ip_address,
            user_agent,
            is_active: true,
        };

        let mut sessions = self.active_sessions.write().await;
        sessions.insert(session.id, session.clone());

        Ok(session)
    }

    /// Validate session
    pub async fn validate_session(&self, session_id: Uuid) -> Option<UserSession> {
        let sessions = self.active_sessions.read().await;
        let session = sessions.get(&session_id)?;

        if !session.is_active || session.expires_at < Utc::now() {
            return None;
        }

        Some(session.clone())
    }

    /// Invalidate session
    pub async fn invalidate_session(&self, session_id: Uuid) -> Result<(), DockerConfigError> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.is_active = false;
            Ok(())
        } else {
            Err(DockerConfigError::Validation("Session not found".to_string()))
        }
    }

    /// Cleanup expired sessions
    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.active_sessions.write().await;
        sessions.retain(|_, session| {
            session.is_active && session.expires_at > Utc::now()
        });
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            username: "user".to_string(),
            email: "user@example.com".to_string(),
            full_name: "Default User".to_string(),
            role: UserRole::Viewer,
            permissions: vec![],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            attributes: HashMap::new(),
        }
    }
}

impl Default for Role {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "default".to_string(),
            description: "Default role".to_string(),
            permissions: vec![],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Default for AccessPolicy {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "default".to_string(),
            description: "Default access policy".to_string(),
            resource: "*".to_string(),
            actions: vec![],
            effect: PolicyEffect::Allow,
            conditions: vec![],
            priority: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}