//! Model selection API types for request/response handling.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single message in a conversation.
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message sender (e.g., "user", "assistant", "system").
    pub role: String,
    /// The content of the message.
    pub content: String,
}

/// Request payload for model selection.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelSelectRequest {
    /// The conversation messages to process.
    pub messages: Vec<Message>,
    /// List of candidate models to choose from.
    pub models: Vec<String>,
    /// Optional user preferences for model selection.
    pub preferences: Option<ModelUserPreferences>,
    /// Optional session ID for tracking.
    pub session_id: Option<String>,
    pub tradeoff: Option<Tradeoff>,        // NEW: Cost/latency/quality optimization
    pub timeout: Option<u32>,               // NEW: Timeout in seconds
    pub default_model: Option<String>,        // NEW: Fallback model
}

/// User preferences for model selection.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelUserPreferences {
    /// Optimization target (quality, speed, cost, or balanced).
    pub optimize_for: Option<OptimizationTarget>,
    /// Maximum tokens for the response.
    pub max_tokens: Option<u32>,
    /// User ID for preference learning.
    pub user_id: Option<String>,
    /// Temperature for response generation.
    pub temperature: Option<f32>,
    /// Custom weights for model scoring.
    pub custom_weights: Option<HashMap<String, f32>>,
}

/// Target metric for optimization during model selection.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationTarget {
    /// Optimize for response quality.
    Quality,
    /// Optimize for response speed.
    Speed,
    /// Optimize for cost efficiency.
    Cost,
    /// Balance between quality, speed, and cost.
    Balanced,
}

/// Response from model selection containing the recommendation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tradeoff {
    Quality,
    Cost,
    Latency,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelSelectResponse {
    /// The recommended model to use.
    pub recommended_model: String,
    /// Confidence score for the recommendation (0.0 to 1.0).
    pub confidence: f64,
    /// Explanation of why this model was selected.
    pub reasoning: String,
    /// Alternative models ranked by suitability.
    pub alternatives: Vec<ModelAlternative>,
    /// Estimated cost for this request.
    pub estimated_cost: Option<f64>,
    /// Estimated latency in milliseconds.
    pub estimated_latency_ms: Option<u32>,
    /// Session ID for tracking.
    pub session_id: Option<String>,
}

/// An alternative model recommendation.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelAlternative {
    /// The model name.
    pub model: String,
    /// Confidence score for this alternative.
    pub confidence: f64,
    /// Estimated cost for this model.
    pub estimated_cost: Option<f64>,
    /// Estimated latency in milliseconds.
    pub estimated_latency_ms: Option<u32>,
}

/// Request payload for submitting feedback.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackRequest {
    /// Session ID for the interaction being rated.
    pub session_id: String,
    /// The model that was used.
    pub model_used: String,
    /// Rating from 1 to 5.
    pub rating: u8,
    /// Type of feedback being provided.
    pub feedback_type: FeedbackType,
    /// Optional comment explaining the rating.
    pub comment: Option<String>,
    /// Additional metadata about the interaction.
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Type of feedback being provided.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    /// Feedback about response quality.
    Quality,
    /// Feedback about response speed.
    Speed,
    /// Feedback about cost efficiency.
    Cost,
    /// Overall feedback.
    Overall,
}

/// Response confirming feedback was processed.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackResponse {
    /// Whether the feedback was processed successfully.
    pub success: bool,
    /// Human-readable message about the result.
    pub message: String,
}

/// Extracted features from a prompt for model selection.
#[derive(Debug, Clone)]
pub struct PromptFeatures {
    /// Length of the prompt in characters.
    pub length: usize,
    /// Complexity score (0.0 to 1.0).
    pub complexity_score: f32,
    /// Categorized domain of the prompt.
    pub domain_category: DomainCategory,
    /// Type of task detected in the prompt.
    pub task_type: TaskType,
    /// Estimated token count.
    pub estimated_tokens: u32,
}

/// Domain category for prompt classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DomainCategory {
    /// General-purpose queries.
    General,
    /// Technical or programming content.
    Technical,
    /// Creative writing tasks.
    Creative,
    /// Data analysis tasks.
    Analytical,
    /// Mathematical problems.
    Mathematical,
    /// Code generation requests.
    CodeGeneration,
    /// Translation tasks.
    Translation,
    /// Summarization tasks.
    Summarization,
    Multilingual,
}

impl std::fmt::Display for DomainCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainCategory::General => write!(f, "general"),
            DomainCategory::Technical => write!(f, "technical"),
            DomainCategory::Creative => write!(f, "creative"),
            DomainCategory::Analytical => write!(f, "analytical"),
            DomainCategory::Mathematical => write!(f, "mathematical"),
            DomainCategory::CodeGeneration => write!(f, "code_generation"),
            DomainCategory::Translation => write!(f, "translation"),
            DomainCategory::Summarization => write!(f, "summarization"),
            DomainCategory::Multilingual => write!(f, "multilingual"),
        }
    }
}

/// Task type detected in a prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Question,
    Instruction,
    Conversation,
    Completion,
    Analysis,
    Generation,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Question => write!(f, "question"),
            TaskType::Instruction => write!(f, "instruction"),
            TaskType::Conversation => write!(f, "conversation"),
            TaskType::Completion => write!(f, "completion"),
            TaskType::Analysis => write!(f, "analysis"),
            TaskType::Generation => write!(f, "generation"),
        }
    }
}

impl PromptFeatures {
    /// Analyzes a list of messages and extracts features for model selection.
    pub fn analyze(messages: &[Message]) -> Self {
        let content = messages
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let length = content.len();
        let complexity_score = Self::calculate_complexity(&content);
        let domain_category = Self::categorize_domain(&content);
        let task_type = Self::identify_task_type(&content);
        let estimated_tokens = Self::estimate_tokens(&content);

        PromptFeatures {
            length,
            complexity_score,
            domain_category,
            task_type,
            estimated_tokens,
        }
    }

    fn calculate_complexity(content: &str) -> f32 {
        // Simple heuristic based on length, vocabulary diversity, and structure
        let words: Vec<&str> = content.split_whitespace().collect();
        let unique_words: std::collections::HashSet<_> = words.iter().collect();
        let vocabulary_diversity = unique_words.len() as f32 / words.len().max(1) as f32;

        // Complexity indicators
        let has_technical_terms = content.contains("API")
            || content.contains("algorithm")
            || content.contains("database")
            || content.contains("implement");

        let has_code = content.contains("function")
            || content.contains("class")
            || content.contains("{}")
            || content.contains("()");

        let has_math = content.contains("calculate")
            || content.contains("equation")
            || content.contains("∫")
            || content.contains("derivative");

        let mut complexity = vocabulary_diversity * 0.4;

        if has_technical_terms {
            complexity += 0.2;
        }
        if has_code {
            complexity += 0.3;
        }
        if has_math {
            complexity += 0.3;
        }
        if words.len() > 100 {
            complexity += 0.1;
        }

        complexity.min(1.0)
    }

    fn categorize_domain(content: &str) -> DomainCategory {
        let content_lower = content.to_lowercase();

        if content_lower.contains("code")
            || content_lower.contains("programming")
            || content_lower.contains("function")
            || content_lower.contains("algorithm")
        {
            DomainCategory::CodeGeneration
        } else if content_lower.contains("math")
            || content_lower.contains("calculate")
            || content_lower.contains("equation")
            || content_lower.contains("solve")
        {
            DomainCategory::Mathematical
        } else if content_lower.contains("analyze")
            || content_lower.contains("data")
            || content_lower.contains("statistics")
            || content_lower.contains("research")
        {
            DomainCategory::Analytical
        } else if content_lower.contains("creative")
            || content_lower.contains("story")
            || content_lower.contains("poem")
            || content_lower.contains("write")
        {
            DomainCategory::Creative
        } else if content_lower.contains("translate") || content_lower.contains("language") {
            DomainCategory::Translation
        } else if content_lower.contains("summarize") || content_lower.contains("summary") {
            DomainCategory::Summarization
        } else if content_lower.contains("technical")
            || content_lower.contains("api")
            || content_lower.contains("system")
            || content_lower.contains("architecture")
        {
            DomainCategory::Technical
        } else {
            DomainCategory::General
        }
    }

    fn identify_task_type(content: &str) -> TaskType {
        let content_lower = content.to_lowercase();

        if content_lower.contains("?") {
            TaskType::Question
        } else if content_lower.starts_with("analyze")
            || content_lower.starts_with("explain")
            || content_lower.starts_with("describe")
            || content_lower.contains("analysis")
        {
            TaskType::Analysis
        } else if content_lower.starts_with("generate")
            || content_lower.starts_with("create")
            || content_lower.starts_with("write")
            || content_lower.contains("generation")
        {
            TaskType::Generation
        } else if content_lower.starts_with("complete") || content_lower.contains("finish") {
            TaskType::Completion
        } else if content_lower.contains("conversation") || content_lower.contains("chat") {
            TaskType::Conversation
        } else {
            TaskType::Instruction
        }
    }

    fn estimate_tokens(content: &str) -> u32 {
        // Rough estimation: ~0.75 tokens per word for English
        let words = content.split_whitespace().count();
        ((words as f32) * 0.75) as u32
    }
}
