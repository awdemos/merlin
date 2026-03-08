use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct ModelSelectionRequest {
    #[validate(length(min = 1, message = "User ID is required"))]
    pub user_id: String,

    #[validate(length(min = 1, message = "Prompt is required"))]
    pub prompt: String,

    #[validate(range(min = 1, max = 8192, message = "Max tokens must be between 1 and 8192"))]
    pub max_tokens: Option<u32>,

    #[validate(range(min = 0.0, max = 2.0, message = "Temperature must be between 0.0 and 2.0"))]
    pub temperature: Option<f64>,

    pub model_preferences: Option<ModelPreferences>,
    pub context: Option<RequestContext>,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct ModelPreferences {
    #[serde(default)]
    pub preferred_models: Vec<String>,

    #[serde(default)]
    pub excluded_models: Vec<String>,

    #[validate(range(min = 0.0, message = "Max cost must be non-negative"))]
    pub max_cost: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestContext {
    pub session_id: Option<String>,
    pub source_application: Option<String>,
    pub user_location: Option<String>,
    pub request_timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelSelectionResponse {
    pub request_id: String,
    pub selected_model: String,
    pub response: String,
    pub tokens_used: u32,
    pub processing_time_ms: u64,
    pub confidence_score: f64,
    pub alternative_models: Vec<AlternativeModel>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlternativeModel {
    pub model_name: String,
    pub confidence_score: f64,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseMetadata {
    pub routing_strategy: String,
    pub user_preferences_applied: bool,
    pub cost_estimates: CostEstimates,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CostEstimates {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceMetrics {
    pub time_to_first_token_ms: Option<u64>,
    pub total_generation_time_ms: u64,
    pub queue_time_ms: Option<u64>,
}

impl ModelSelectionRequest {
    pub fn new(user_id: String, prompt: String) -> Self {
        Self {
            user_id,
            prompt,
            max_tokens: Some(1000),
            temperature: Some(0.7),
            model_preferences: None,
            context: None,
        }
    }

    pub fn with_preferences(mut self, preferences: ModelPreferences) -> Self {
        self.model_preferences = Some(preferences);
        self
    }

    pub fn with_context(mut self, context: RequestContext) -> Self {
        self.context = Some(context);
        self
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

impl ModelSelectionResponse {
    pub fn new(
        selected_model: String,
        response: String,
        tokens_used: u32,
        processing_time_ms: u64,
        confidence_score: f64,
    ) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            selected_model,
            response,
            tokens_used,
            processing_time_ms,
            confidence_score,
            alternative_models: Vec::new(),
            metadata: ResponseMetadata {
                routing_strategy: "intelligent".to_string(),
                user_preferences_applied: false,
                cost_estimates: CostEstimates {
                    input_tokens: 0,
                    output_tokens: tokens_used,
                    estimated_cost_usd: 0.0,
                },
                performance_metrics: PerformanceMetrics {
                    time_to_first_token_ms: None,
                    total_generation_time_ms: processing_time_ms,
                    queue_time_ms: None,
                },
            },
        }
    }

    pub fn with_alternatives(mut self, alternatives: Vec<AlternativeModel>) -> Self {
        self.alternative_models = alternatives;
        self
    }

    pub fn with_routing_strategy(mut self, strategy: String) -> Self {
        self.metadata.routing_strategy = strategy;
        self
    }

    pub fn with_user_preferences_applied(mut self, applied: bool) -> Self {
        self.metadata.user_preferences_applied = applied;
        self
    }

    pub fn with_cost_estimates(mut self, estimates: CostEstimates) -> Self {
        self.metadata.cost_estimates = estimates;
        self
    }
}

impl AlternativeModel {
    pub fn new(model_name: String, confidence_score: f64, reason: String) -> Self {
        Self {
            model_name,
            confidence_score,
            reason,
        }
    }
}

impl CostEstimates {
    pub fn new(input_tokens: u32, output_tokens: u32, estimated_cost_usd: f64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            estimated_cost_usd,
        }
    }
}

impl PerformanceMetrics {
    pub fn new(total_generation_time_ms: u64) -> Self {
        Self {
            time_to_first_token_ms: None,
            total_generation_time_ms,
            queue_time_ms: None,
        }
    }

    pub fn with_time_to_first_token(mut self, time_ms: u64) -> Self {
        self.time_to_first_token_ms = Some(time_ms);
        self
    }

    pub fn with_queue_time(mut self, queue_time_ms: u64) -> Self {
        self.queue_time_ms = Some(queue_time_ms);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_selection_request_creation() {
        let request = ModelSelectionRequest::new("user123".to_string(), "Test prompt".to_string());
        assert_eq!(request.user_id, "user123");
        assert_eq!(request.prompt, "Test prompt");
        assert_eq!(request.max_tokens, Some(1000));
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_model_selection_request_validation() {
        let mut request = ModelSelectionRequest::new("".to_string(), "Test prompt".to_string());
        request.max_tokens = Some(0); // Invalid

        let result = request.validate_request();
        assert!(result.is_err());
    }

    #[test]
    fn test_model_selection_response_creation() {
        let response = ModelSelectionResponse::new(
            "gpt-4".to_string(),
            "Test response".to_string(),
            100,
            50,
            0.95,
        );

        assert!(!response.request_id.is_empty());
        assert_eq!(response.selected_model, "gpt-4");
        assert_eq!(response.tokens_used, 100);
        assert_eq!(response.processing_time_ms, 50);
        assert_eq!(response.confidence_score, 0.95);
    }

    #[test]
    fn test_alternative_model_creation() {
        let alternative = AlternativeModel::new(
            "claude-3".to_string(),
            0.85,
            "Second best choice".to_string(),
        );

        assert_eq!(alternative.model_name, "claude-3");
        assert_eq!(alternative.confidence_score, 0.85);
        assert_eq!(alternative.reason, "Second best choice");
    }

    #[test]
    fn test_cost_estimates_creation() {
        let estimates = CostEstimates::new(50, 100, 0.015);

        assert_eq!(estimates.input_tokens, 50);
        assert_eq!(estimates.output_tokens, 100);
        assert_eq!(estimates.estimated_cost_usd, 0.015);
    }

    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics::new(100);

        assert_eq!(metrics.total_generation_time_ms, 100);
        assert!(metrics.time_to_first_token_ms.is_none());

        let with_ttfb = metrics.with_time_to_first_token(25);
        assert_eq!(with_ttfb.time_to_first_token_ms, Some(25));
    }
}