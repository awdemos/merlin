// src/api/model_select.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelSelectRequest {
    pub messages: Vec<Message>,
    pub models: Vec<String>,
    pub preferences: Option<ModelUserPreferences>,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelUserPreferences {
    pub optimize_for: Option<OptimizationTarget>,
    pub max_tokens: Option<u32>,
    pub user_id: Option<String>,
    pub temperature: Option<f32>,
    pub custom_weights: Option<HashMap<String, f32>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationTarget {
    Quality,
    Speed,
    Cost,
    Balanced,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelSelectResponse {
    pub recommended_model: String,
    pub confidence: f64,
    pub reasoning: String,
    pub alternatives: Vec<ModelAlternative>,
    pub estimated_cost: Option<f64>,
    pub estimated_latency_ms: Option<u32>,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelAlternative {
    pub model: String,
    pub confidence: f64,
    pub estimated_cost: Option<f64>,
    pub estimated_latency_ms: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackRequest {
    pub session_id: String,
    pub model_used: String,
    pub rating: u8, // 1-5 scale
    pub feedback_type: FeedbackType,
    pub comment: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    Quality,
    Speed,
    Cost,
    Overall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackResponse {
    pub success: bool,
    pub message: String,
}

// Prompt analysis structures
#[derive(Debug, Clone)]
pub struct PromptFeatures {
    pub length: usize,
    pub complexity_score: f32,
    pub domain_category: DomainCategory,
    pub task_type: TaskType,
    pub estimated_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DomainCategory {
    General,
    Technical,
    Creative,
    Analytical,
    Mathematical,
    CodeGeneration,
    Translation,
    Summarization,
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
        }
    }
}

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
        let has_technical_terms = content.contains("API") || 
                                content.contains("algorithm") || 
                                content.contains("database") ||
                                content.contains("implement");
        
        let has_code = content.contains("function") || 
                      content.contains("class") || 
                      content.contains("{}") ||
                      content.contains("()");
        
        let has_math = content.contains("calculate") || 
                      content.contains("equation") || 
                      content.contains("∫") ||
                      content.contains("derivative");
        
        let mut complexity = vocabulary_diversity * 0.4;
        
        if has_technical_terms { complexity += 0.2; }
        if has_code { complexity += 0.3; }
        if has_math { complexity += 0.3; }
        if words.len() > 100 { complexity += 0.1; }
        
        complexity.min(1.0)
    }

    fn categorize_domain(content: &str) -> DomainCategory {
        let content_lower = content.to_lowercase();
        
        if content_lower.contains("code") || content_lower.contains("programming") || 
           content_lower.contains("function") || content_lower.contains("algorithm") {
            DomainCategory::CodeGeneration
        } else if content_lower.contains("math") || content_lower.contains("calculate") ||
                 content_lower.contains("equation") || content_lower.contains("solve") {
            DomainCategory::Mathematical
        } else if content_lower.contains("analyze") || content_lower.contains("data") ||
                 content_lower.contains("statistics") || content_lower.contains("research") {
            DomainCategory::Analytical
        } else if content_lower.contains("creative") || content_lower.contains("story") ||
                 content_lower.contains("poem") || content_lower.contains("write") {
            DomainCategory::Creative
        } else if content_lower.contains("translate") || content_lower.contains("language") {
            DomainCategory::Translation
        } else if content_lower.contains("summarize") || content_lower.contains("summary") {
            DomainCategory::Summarization
        } else if content_lower.contains("technical") || content_lower.contains("api") ||
                 content_lower.contains("system") || content_lower.contains("architecture") {
            DomainCategory::Technical
        } else {
            DomainCategory::General
        }
    }

    fn identify_task_type(content: &str) -> TaskType {
        let content_lower = content.to_lowercase();
        
        if content_lower.contains("?") {
            TaskType::Question
        } else if content_lower.starts_with("analyze") || content_lower.starts_with("explain") ||
                 content_lower.starts_with("describe") || content_lower.contains("analysis") {
            TaskType::Analysis
        } else if content_lower.starts_with("generate") || content_lower.starts_with("create") ||
                 content_lower.starts_with("write") || content_lower.contains("generation") {
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
