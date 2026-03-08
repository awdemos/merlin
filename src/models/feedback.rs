use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct FeedbackSubmission {
    #[validate(length(min = 1, message = "User ID is required"))]
    pub user_id: String,

    #[validate(length(min = 1, message = "Model name is required"))]
    pub model_name: String,

    #[validate(range(min = 1, max = 5, message = "Rating must be between 1 and 5"))]
    pub rating: u8,

    pub request_id: Option<String>,
    pub feedback_text: Option<String>,

    #[validate(custom = "validate_category")]
    pub category: FeedbackCategory,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum FeedbackCategory {
    Accuracy,
    Helpfulness,
    Speed,
    Cost,
    Creativity,
    TechnicalQuality,
    UserExperience,
    Other,
}

impl FeedbackCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeedbackCategory::Accuracy => "Accuracy",
            FeedbackCategory::Helpfulness => "Helpfulness",
            FeedbackCategory::Speed => "Speed",
            FeedbackCategory::Cost => "Cost",
            FeedbackCategory::Creativity => "Creativity",
            FeedbackCategory::TechnicalQuality => "TechnicalQuality",
            FeedbackCategory::UserExperience => "UserExperience",
            FeedbackCategory::Other => "Other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Accuracy" => Some(FeedbackCategory::Accuracy),
            "Helpfulness" => Some(FeedbackCategory::Helpfulness),
            "Speed" => Some(FeedbackCategory::Speed),
            "Cost" => Some(FeedbackCategory::Cost),
            "Creativity" => Some(FeedbackCategory::Creativity),
            "TechnicalQuality" => Some(FeedbackCategory::TechnicalQuality),
            "UserExperience" => Some(FeedbackCategory::UserExperience),
            "Other" => Some(FeedbackCategory::Other),
            _ => None,
        }
    }

    pub fn all_categories() -> Vec<&'static str> {
        vec![
            "Accuracy",
            "Helpfulness",
            "Speed",
            "Cost",
            "Creativity",
            "TechnicalQuality",
            "UserExperience",
            "Other",
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeedbackResponse {
    pub id: String,
    pub user_id: String,
    pub request_id: Option<String>,
    pub model_name: String,
    pub rating: u8,
    pub feedback_text: Option<String>,
    pub category: FeedbackCategory,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: FeedbackMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeedbackMetadata {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub session_id: Option<String>,
    pub source: String,
    pub processed: bool,
    pub analysis_result: Option<FeedbackAnalysis>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeedbackAnalysis {
    pub sentiment_score: f64,
    pub key_themes: Vec<String>,
    pub action_items: Vec<String>,
    pub severity_level: Option<String>,
}

fn validate_category(category: &FeedbackCategory) -> Result<(), validator::ValidationError> {
    // Custom validation can be added here if needed
    Ok(())
}

impl FeedbackSubmission {
    pub fn new(
        user_id: String,
        model_name: String,
        rating: u8,
        category: FeedbackCategory,
    ) -> Self {
        Self {
            user_id,
            model_name,
            rating,
            request_id: None,
            feedback_text: None,
            category,
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_feedback_text(mut self, feedback_text: String) -> Self {
        self.feedback_text = Some(feedback_text);
        self
    }

    pub fn validate_submission(&self) -> Result<(), String> {
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

    pub fn is_positive(&self) -> bool {
        self.rating >= 4
    }

    pub fn is_negative(&self) -> bool {
        self.rating <= 2
    }
}

impl FeedbackResponse {
    pub fn new(
        user_id: String,
        model_name: String,
        rating: u8,
        category: FeedbackCategory,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            request_id: None,
            model_name,
            rating,
            feedback_text: None,
            category,
            created_at: now.clone(),
            updated_at: now,
            metadata: FeedbackMetadata {
                ip_address: None,
                user_agent: None,
                session_id: None,
                source: "api".to_string(),
                processed: false,
                analysis_result: None,
            },
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_feedback_text(mut self, feedback_text: String) -> Self {
        self.feedback_text = Some(feedback_text);
        self
    }

    pub fn with_metadata(mut self, metadata: FeedbackMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.metadata.session_id = Some(session_id);
        self
    }

    pub fn mark_processed(mut self) -> Self {
        self.metadata.processed = true;
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self
    }

    pub fn with_analysis(mut self, analysis: FeedbackAnalysis) -> Self {
        self.metadata.analysis_result = Some(analysis);
        self
    }
}

impl FeedbackMetadata {
    pub fn new() -> Self {
        Self {
            ip_address: None,
            user_agent: None,
            session_id: None,
            source: "api".to_string(),
            processed: false,
            analysis_result: None,
        }
    }

    pub fn with_ip_address(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }
}

impl FeedbackAnalysis {
    pub fn new(sentiment_score: f64) -> Self {
        Self {
            sentiment_score,
            key_themes: Vec::new(),
            action_items: Vec::new(),
            severity_level: None,
        }
    }

    pub fn with_key_themes(mut self, themes: Vec<String>) -> Self {
        self.key_themes = themes;
        self
    }

    pub fn with_action_items(mut self, items: Vec<String>) -> Self {
        self.action_items = items;
        self
    }

    pub fn with_severity_level(mut self, level: String) -> Self {
        self.severity_level = Some(level);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_submission_creation() {
        let feedback = FeedbackSubmission::new(
            "user123".to_string(),
            "gpt-4".to_string(),
            5,
            FeedbackCategory::Helpfulness,
        );

        assert_eq!(feedback.user_id, "user123");
        assert_eq!(feedback.model_name, "gpt-4");
        assert_eq!(feedback.rating, 5);
        assert_eq!(feedback.category, FeedbackCategory::Helpfulness);
    }

    #[test]
    fn test_feedback_submission_validation() {
        let mut feedback = FeedbackSubmission::new(
            "".to_string(), // Invalid user_id
            "gpt-4".to_string(),
            0, // Invalid rating
            FeedbackCategory::Helpfulness,
        );

        let result = feedback.validate_submission();
        assert!(result.is_err());
    }

    #[test]
    fn test_feedback_rating_classification() {
        let positive = FeedbackSubmission::new(
            "user123".to_string(),
            "gpt-4".to_string(),
            4,
            FeedbackCategory::Helpfulness,
        );
        assert!(positive.is_positive());
        assert!(!positive.is_negative());

        let negative = FeedbackSubmission::new(
            "user123".to_string(),
            "gpt-4".to_string(),
            2,
            FeedbackCategory::Helpfulness,
        );
        assert!(negative.is_negative());
        assert!(!negative.is_positive());
    }

    #[test]
    fn test_feedback_category_conversion() {
        assert_eq!(FeedbackCategory::Accuracy.as_str(), "Accuracy");
        assert_eq!(FeedbackCategory::from_str("Accuracy"), Some(FeedbackCategory::Accuracy));
        assert_eq!(FeedbackCategory::from_str("Invalid"), None);
    }

    #[test]
    fn test_feedback_response_creation() {
        let response = FeedbackResponse::new(
            "user123".to_string(),
            "gpt-4".to_string(),
            5,
            FeedbackCategory::Accuracy,
        );

        assert!(!response.id.is_empty());
        assert_eq!(response.user_id, "user123");
        assert_eq!(response.model_name, "gpt-4");
        assert_eq!(response.rating, 5);
        assert_eq!(response.category, FeedbackCategory::Accuracy);
        assert!(!response.metadata.processed);
    }

    #[test]
    fn test_feedback_metadata_creation() {
        let metadata = FeedbackMetadata::new()
            .with_ip_address("192.168.1.1".to_string())
            .with_session_id("session123".to_string());

        assert_eq!(metadata.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(metadata.session_id, Some("session123".to_string()));
        assert_eq!(metadata.source, "api");
    }

    #[test]
    fn test_feedback_analysis_creation() {
        let analysis = FeedbackAnalysis::new(0.8)
            .with_key_themes(vec!["helpful".to_string(), "accurate".to_string()])
            .with_action_items(vec!["improve speed".to_string()]);

        assert_eq!(analysis.sentiment_score, 0.8);
        assert_eq!(analysis.key_themes, vec!["helpful", "accurate"]);
        assert_eq!(analysis.action_items, vec!["improve speed"]);
    }
}