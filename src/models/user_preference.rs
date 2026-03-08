use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct UserPreference {
    #[validate(length(min = 1, message = "User ID is required"))]
    pub user_id: String,

    #[validate(length(min = 1, message = "Preference key is required"))]
    pub preference_key: String,

    #[validate(custom = "validate_preference_value")]
    pub preference_value: serde_json::Value,

    #[validate(custom = "validate_category")]
    pub category: PreferenceCategory,

    pub version: Option<u32>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum PreferenceCategory {
    ModelSelection,
    ResponseFormatting,
    UserInterface,
    NotificationSettings,
    PrivacySettings,
    CostManagement,
    PerformanceSettings,
    Other,
}

impl PreferenceCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PreferenceCategory::ModelSelection => "ModelSelection",
            PreferenceCategory::ResponseFormatting => "ResponseFormatting",
            PreferenceCategory::UserInterface => "UserInterface",
            PreferenceCategory::NotificationSettings => "NotificationSettings",
            PreferenceCategory::PrivacySettings => "PrivacySettings",
            PreferenceCategory::CostManagement => "CostManagement",
            PreferenceCategory::PerformanceSettings => "PerformanceSettings",
            PreferenceCategory::Other => "Other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ModelSelection" => Some(PreferenceCategory::ModelSelection),
            "ResponseFormatting" => Some(PreferenceCategory::ResponseFormatting),
            "UserInterface" => Some(PreferenceCategory::UserInterface),
            "NotificationSettings" => Some(PreferenceCategory::NotificationSettings),
            "PrivacySettings" => Some(PreferenceCategory::PrivacySettings),
            "CostManagement" => Some(PreferenceCategory::CostManagement),
            "PerformanceSettings" => Some(PreferenceCategory::PerformanceSettings),
            "Other" => Some(PreferenceCategory::Other),
            _ => None,
        }
    }

    pub fn all_categories() -> Vec<&'static str> {
        vec![
            "ModelSelection",
            "ResponseFormatting",
            "UserInterface",
            "NotificationSettings",
            "PrivacySettings",
            "CostManagement",
            "PerformanceSettings",
            "Other",
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPreferenceResponse {
    pub id: String,
    pub user_id: String,
    pub preference_key: String,
    pub preference_value: serde_json::Value,
    pub category: PreferenceCategory,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: PreferenceMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreferenceMetadata {
    pub last_accessed_at: Option<String>,
    pub access_count: u32,
    pub is_system_default: bool,
    pub can_be_overridden: bool,
    pub schema_version: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct PreferenceUpdateRequest {
    #[validate(length(min = 1, message = "User ID is required"))]
    pub user_id: String,

    #[validate(length(min = 1, message = "Preference key is required"))]
    pub preference_key: String,

    #[validate(custom = "validate_preference_value")]
    pub preference_value: serde_json::Value,

    #[validate(custom = "validate_category")]
    pub category: PreferenceCategory,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreferenceDeleteRequest {
    #[validate(length(min = 1, message = "User ID is required"))]
    pub user_id: String,

    #[validate(length(min = 1, message = "Preference key is required"))]
    pub preference_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreferenceDeleteResponse {
    pub user_id: String,
    pub preference_key: String,
    pub deleted: bool,
    pub deleted_at: String,
    pub metadata: PreferenceMetadata,
}

fn validate_preference_value(value: &serde_json::Value) -> Result<(), validator::ValidationError> {
    // Validate that the preference value is not null
    if value.is_null() {
        let mut error = validator::ValidationError::new("invalid_value");
        error.message = Some("Preference value cannot be null".into());
        return Err(error);
    }

    // Validate string values are not empty
    if let Some(s) = value.as_str() {
        if s.trim().is_empty() {
            let mut error = validator::ValidationError::new("invalid_value");
            error.message = Some("String preference value cannot be empty".into());
            return Err(error);
        }
    }

    // Validate array values are not empty
    if let Some(arr) = value.as_array() {
        if arr.is_empty() {
            let mut error = validator::ValidationError::new("invalid_value");
            error.message = Some("Array preference value cannot be empty".into());
            return Err(error);
        }
    }

    Ok(())
}

fn validate_category(category: &PreferenceCategory) -> Result<(), validator::ValidationError> {
    // Custom validation can be added here if needed
    Ok(())
}

impl UserPreference {
    pub fn new(
        user_id: String,
        preference_key: String,
        preference_value: serde_json::Value,
        category: PreferenceCategory,
    ) -> Self {
        Self {
            user_id,
            preference_key,
            preference_value,
            category,
            version: Some(1),
            created_at: None,
            updated_at: None,
        }
    }

    pub fn with_version(mut self, version: u32) -> Self {
        self.version = Some(version);
        self
    }

    pub fn with_timestamps(mut self, created_at: String, updated_at: String) -> Self {
        self.created_at = Some(created_at);
        self.updated_at = Some(updated_at);
        self
    }

    pub fn validate_preference(&self) -> Result<(), String> {
        if let Err(errors) = self.validate() {
            let error_messages: Vec<String> = errors
                .field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.into_iter().map(move |error| {
                        format!("{}: {}", field, error.message.as_ref().unwrap_or(&"invalid value".into()))
                    })
                })
                .collect();
            return Err(error_messages.join(", "));
        }
        Ok(())
    }

    pub fn increment_version(&mut self) {
        self.version = self.version.map(|v| v + 1).or(Some(1));
    }

    pub fn is_model_preference(&self) -> bool {
        matches!(self.category, PreferenceCategory::ModelSelection)
    }

    pub fn is_formatting_preference(&self) -> bool {
        matches!(self.category, PreferenceCategory::ResponseFormatting)
    }
}

impl UserPreferenceResponse {
    pub fn new(
        user_id: String,
        preference_key: String,
        preference_value: serde_json::Value,
        category: PreferenceCategory,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            preference_key,
            preference_value,
            category,
            version: 1,
            created_at: now.clone(),
            updated_at: now,
            metadata: PreferenceMetadata {
                last_accessed_at: None,
                access_count: 0,
                is_system_default: false,
                can_be_overridden: true,
                schema_version: "1.0".to_string(),
            },
        }
    }

    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    pub fn with_metadata(mut self, metadata: PreferenceMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_timestamps(mut self, created_at: String, updated_at: String) -> Self {
        self.created_at = created_at;
        self.updated_at = updated_at;
        self
    }

    pub fn mark_accessed(mut self) -> Self {
        self.metadata.last_accessed_at = Some(chrono::Utc::now().to_rfc3339());
        self.metadata.access_count += 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self
    }

    pub fn increment_version(&mut self) {
        self.version += 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

impl PreferenceMetadata {
    pub fn new() -> Self {
        Self {
            last_accessed_at: None,
            access_count: 0,
            is_system_default: false,
            can_be_overridden: true,
            schema_version: "1.0".to_string(),
        }
    }

    pub fn system_default() -> Self {
        Self {
            last_accessed_at: None,
            access_count: 0,
            is_system_default: true,
            can_be_overridden: true,
            schema_version: "1.0".to_string(),
        }
    }

    pub fn with_last_accessed(mut self, timestamp: String) -> Self {
        self.last_accessed_at = Some(timestamp);
        self
    }

    pub fn with_access_count(mut self, count: u32) -> Self {
        self.access_count = count;
        self
    }

    pub fn with_schema_version(mut self, version: String) -> Self {
        self.schema_version = version;
        self
    }
}

impl PreferenceUpdateRequest {
    pub fn new(
        user_id: String,
        preference_key: String,
        preference_value: serde_json::Value,
        category: PreferenceCategory,
    ) -> Self {
        Self {
            user_id,
            preference_key,
            preference_value,
            category,
        }
    }

    pub fn validate_request(&self) -> Result<(), String> {
        if let Err(errors) = self.validate() {
            let error_messages: Vec<String> = errors
                .field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.into_iter().map(move |error| {
                        format!("{}: {}", field, error.message.as_ref().unwrap_or(&"invalid value".into()))
                    })
                })
                .collect();
            return Err(error_messages.join(", "));
        }
        Ok(())
    }
}

impl PreferenceDeleteRequest {
    pub fn new(user_id: String, preference_key: String) -> Self {
        Self {
            user_id,
            preference_key,
        }
    }

    pub fn validate_request(&self) -> Result<(), String> {
        if let Err(errors) = self.validate() {
            let error_messages: Vec<String> = errors
                .field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.into_iter().map(move |error| {
                        format!("{}: {}", field, error.message.as_ref().unwrap_or(&"invalid value".into()))
                    })
                })
                .collect();
            return Err(error_messages.join(", "));
        }
        Ok(())
    }
}

impl PreferenceDeleteResponse {
    pub fn new(user_id: String, preference_key: String, deleted: bool) -> Self {
        Self {
            user_id,
            preference_key,
            deleted,
            deleted_at: chrono::Utc::now().to_rfc3339(),
            metadata: PreferenceMetadata::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: PreferenceMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_preference_creation() {
        let preference = UserPreference::new(
            "user123".to_string(),
            "preferred_models".to_string(),
            serde_json::json!(["gpt-4", "claude-3"]),
            PreferenceCategory::ModelSelection,
        );

        assert_eq!(preference.user_id, "user123");
        assert_eq!(preference.preference_key, "preferred_models");
        assert_eq!(preference.category, PreferenceCategory::ModelSelection);
        assert_eq!(preference.version, Some(1));
    }

    #[test]
    fn test_preference_validation() {
        let valid_preference = UserPreference::new(
            "user123".to_string(),
            "style".to_string(),
            serde_json::json!("detailed"),
            PreferenceCategory::ResponseFormatting,
        );

        assert!(valid_preference.validate_preference().is_ok());

        let invalid_preference = UserPreference::new(
            "".to_string(), // Invalid user_id
            "style".to_string(),
            serde_json::json!(""), // Invalid empty string
            PreferenceCategory::ResponseFormatting,
        );

        assert!(invalid_preference.validate_preference().is_err());
    }

    #[test]
    fn test_preference_category_conversion() {
        assert_eq!(PreferenceCategory::ModelSelection.as_str(), "ModelSelection");
        assert_eq!(PreferenceCategory::from_str("ModelSelection"), Some(PreferenceCategory::ModelSelection));
        assert_eq!(PreferenceCategory::from_str("Invalid"), None);
    }

    #[test]
    fn test_preference_response_creation() {
        let response = UserPreferenceResponse::new(
            "user123".to_string(),
            "theme".to_string(),
            serde_json::json!("dark"),
            PreferenceCategory::UserInterface,
        );

        assert!(!response.id.is_empty());
        assert_eq!(response.user_id, "user123");
        assert_eq!(response.version, 1);
        assert_eq!(response.metadata.access_count, 0);
    }

    #[test]
    fn test_preference_metadata_creation() {
        let metadata = PreferenceMetadata::system_default()
            .with_access_count(5)
            .with_schema_version("2.0".to_string());

        assert!(metadata.is_system_default);
        assert_eq!(metadata.access_count, 5);
        assert_eq!(metadata.schema_version, "2.0");
    }

    #[test]
    fn test_preference_update_request() {
        let request = PreferenceUpdateRequest::new(
            "user123".to_string(),
            "max_tokens".to_string(),
            serde_json::json!(1000),
            PreferenceCategory::ModelSelection,
        );

        assert!(request.validate_request().is_ok());
        assert_eq!(request.user_id, "user123");
    }

    #[test]
    fn test_preference_delete_request() {
        let request = PreferenceDeleteRequest::new(
            "user123".to_string(),
            "old_preference".to_string(),
        );

        assert!(request.validate_request().is_ok());
        assert_eq!(request.user_id, "user123");
        assert_eq!(request.preference_key, "old_preference");
    }

    #[test]
    fn test_preference_classification() {
        let model_pref = UserPreference::new(
            "user123".to_string(),
            "models".to_string(),
            serde_json!(["gpt-4"]),
            PreferenceCategory::ModelSelection,
        );
        assert!(model_pref.is_model_preference());
        assert!(!model_pref.is_formatting_preference());

        let format_pref = UserPreference::new(
            "user123".to_string(),
            "style".to_string(),
            serde_json!("concise"),
            PreferenceCategory::ResponseFormatting,
        );
        assert!(format_pref.is_formatting_preference());
        assert!(!format_pref.is_model_preference());
    }
}