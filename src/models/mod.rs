pub mod container_state;
pub mod deployment_environment;
pub mod docker_config;
pub mod health_check_config;
pub mod network_config;
pub mod resource_limits;
pub mod security_profile;
pub mod security_scan_config;
pub mod tmpfs_mount;
pub mod volume_mount;

// API models
pub mod model_selection;
pub mod feedback;
pub mod user_preference;
pub mod api_response;

pub use container_state::*;
pub use deployment_environment::*;
pub use docker_config::*;
pub use health_check_config::*;
pub use network_config::*;
pub use resource_limits::*;
pub use security_profile::*;
pub use security_scan_config::*;
pub use tmpfs_mount::*;
pub use volume_mount::*;

// Re-export API models
pub use model_selection::*;
pub use feedback::*;
pub use user_preference::*;
pub use api_response::*;

// Type aliases for common response types
pub type ModelSelectionResult = APIResponse<ModelSelectionResponse>;
pub type FeedbackResult = APIResponse<FeedbackResponse>;
pub type PreferenceResult = APIResponse<UserPreferenceResponse>;
pub type PreferenceDeleteResult = APIResponse<PreferenceDeleteResponse>;

// Helper functions for creating standard responses
pub fn create_success_response<T>(data: T) -> APIResponse<T> {
    APIResponse::success(data)
}

pub fn create_error_response(error: APIError) -> APIResponse<String> {
    APIResponse::error(error)
}

pub fn create_validation_error(message: &str, field: Option<&str>) -> APIError {
    APIError::validation(
        message.to_string(),
        field.map(|s| s.to_string()),
    )
}

pub fn create_not_found_error(resource: &str) -> APIError {
    APIError::new(
        ErrorCode::NotFoundError,
        format!("{} not found", resource),
        None,
    )
}

pub fn create_conflict_error(message: &str) -> APIError {
    APIError::new(
        ErrorCode::ConflictError,
        message.to_string(),
        None,
    )
}

pub fn create_internal_error(message: &str) -> APIError {
    APIError::new(
        ErrorCode::InternalServerError,
        message.to_string(),
        None,
    )
}

// Validation helpers
pub fn validate_required_string(value: &str, field_name: &str) -> Result<(), APIError> {
    if value.trim().is_empty() {
        Err(create_validation_error(&format!("{} is required", field_name), Some(field_name)))
    } else {
        Ok(())
    }
}

pub fn validate_rating(rating: u8) -> Result<(), APIError> {
    if !(1..=5).contains(&rating) {
        Err(create_validation_error("Rating must be between 1 and 5", Some("rating")))
    } else {
        Ok(())
    }
}

pub fn validate_max_tokens(max_tokens: u32) -> Result<(), APIError> {
    if max_tokens == 0 || max_tokens > 8192 {
        Err(create_validation_error("Max tokens must be between 1 and 8192", Some("max_tokens")))
    } else {
        Ok(())
    }
}

pub fn validate_temperature(temperature: f64) -> Result<(), APIError> {
    if !(0.0..=2.0).contains(&temperature) {
        Err(create_validation_error("Temperature must be between 0.0 and 2.0", Some("temperature")))
    } else {
        Ok(())
    }
}

// Model validation helpers
pub fn validate_model_name(model_name: &str) -> Result<(), APIError> {
    let valid_models = ["gpt-4", "gpt-3.5-turbo", "claude-3", "claude-2", "gemini-pro"];
    if !valid_models.contains(&model_name) {
        Err(create_validation_error(
            &format!("Invalid model name: {}", model_name),
            Some("model_name"),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_preference_category(category: &str) -> Result<(), APIError> {
    if PreferenceCategory::from_str(category).is_none() {
        Err(create_validation_error(
            &format!("Invalid preference category: {}", category),
            Some("category"),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_feedback_category(category: &str) -> Result<(), APIError> {
    if FeedbackCategory::from_str(category).is_none() {
        Err(create_validation_error(
            &format!("Invalid feedback category: {}", category),
            Some("category"),
        ))
    } else {
        Ok(())
    }
}

// Utility functions for working with JSON values
pub fn validate_preference_value(value: &serde_json::Value) -> Result<(), APIError> {
    if value.is_null() {
        return Err(create_validation_error("Preference value cannot be null", Some("preference_value")));
    }

    if let Some(s) = value.as_str() {
        if s.trim().is_empty() {
            return Err(create_validation_error("String preference value cannot be empty", Some("preference_value")));
        }
    }

    if let Some(arr) = value.as_array() {
        if arr.is_empty() {
            return Err(create_validation_error("Array preference value cannot be empty", Some("preference_value")));
        }
    }

    Ok(())
}

// Configuration constants
pub const DEFAULT_MAX_TOKENS: u32 = 1000;
pub const DEFAULT_TEMPERATURE: f64 = 0.7;
pub const MAX_PREFERENCE_VALUE_LENGTH: usize = 10000;
pub const MAX_FEEDBACK_TEXT_LENGTH: usize = 2000;

// Helper functions for default values
pub fn default_max_tokens() -> u32 {
    DEFAULT_MAX_TOKENS
}

pub fn default_temperature() -> f64 {
    DEFAULT_TEMPERATURE
}

pub fn default_page_size() -> u32 {
    20
}

pub fn max_page_size() -> u32 {
    100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_required_string() {
        assert!(validate_required_string("test", "field").is_ok());
        assert!(validate_required_string("", "field").is_err());
        assert!(validate_required_string("   ", "field").is_err());
    }

    #[test]
    fn test_validate_rating() {
        assert!(validate_rating(3).is_ok());
        assert!(validate_rating(1).is_ok());
        assert!(validate_rating(5).is_ok());
        assert!(validate_rating(0).is_err());
        assert!(validate_rating(6).is_err());
    }

    #[test]
    fn test_validate_max_tokens() {
        assert!(validate_max_tokens(100).is_ok());
        assert!(validate_max_tokens(8192).is_ok());
        assert!(validate_max_tokens(0).is_err());
        assert!(validate_max_tokens(8193).is_err());
    }

    #[test]
    fn test_validate_temperature() {
        assert!(validate_temperature(0.5).is_ok());
        assert!(validate_temperature(0.0).is_ok());
        assert!(validate_temperature(2.0).is_ok());
        assert!(validate_temperature(-0.1).is_err());
        assert!(validate_temperature(2.1).is_err());
    }

    #[test]
    fn test_validate_model_name() {
        assert!(validate_model_name("gpt-4").is_ok());
        assert!(validate_model_name("claude-3").is_ok());
        assert!(validate_model_name("invalid-model").is_err());
    }

    #[test]
    fn test_validate_preference_category() {
        assert!(validate_preference_category("ModelSelection").is_ok());
        assert!(validate_preference_category("ResponseFormatting").is_ok());
        assert!(validate_preference_category("InvalidCategory").is_err());
    }

    #[test]
    fn test_validate_feedback_category() {
        assert!(validate_feedback_category("Accuracy").is_ok());
        assert!(validate_feedback_category("Helpfulness").is_ok());
        assert!(validate_feedback_category("InvalidCategory").is_err());
    }

    #[test]
    fn test_validate_preference_value() {
        assert!(validate_preference_value(&serde_json::json!("test")).is_ok());
        assert!(validate_preference_value(&serde_json!(["test"])).is_ok());
        assert!(validate_preference_value(&serde_json::json!(null)).is_err());
        assert!(validate_preference_value(&serde_json::json!("")).is_err());
        assert!(validate_preference_value(&serde_json::json!([])).is_err());
    }

    #[test]
    fn test_default_values() {
        assert_eq!(default_max_tokens(), DEFAULT_MAX_TOKENS);
        assert_eq!(default_temperature(), DEFAULT_TEMPERATURE);
        assert_eq!(default_page_size(), 20);
        assert_eq!(max_page_size(), 100);
    }

    #[test]
    fn test_error_creation_helpers() {
        let error = create_validation_error("Test message", Some("field"));
        assert_eq!(error.message, "Test message");
        assert_eq!(error.code, "VALIDATION_ERROR");

        let error = create_not_found_error("test resource");
        assert_eq!(error.message, "test resource not found");
        assert_eq!(error.code, "NOT_FOUND");

        let error = create_conflict_error("Test conflict");
        assert_eq!(error.message, "Test conflict");
        assert_eq!(error.code, "CONFLICT");

        let error = create_internal_error("Test error");
        assert_eq!(error.message, "Test error");
        assert_eq!(error.code, "INTERNAL_SERVER_ERROR");
    }
}