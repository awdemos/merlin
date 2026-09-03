use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Reason the model stopped generating.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    #[default]
    Unknown,
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
}

impl FromStr for StopReason {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "stop" => StopReason::Stop,
            "length" => StopReason::Length,
            "tool_calls" => StopReason::ToolCalls,
            "content_filter" => StopReason::ContentFilter,
            _ => StopReason::Unknown,
        })
    }
}

/// Token usage for a completed request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
}

/// A completed, aggregated LLM response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggLlmResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<super::conversation::ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<super::tools::ToolCall>>,
    pub usage: Usage,
    pub stop_reason: StopReason,
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub preservation: super::envelope::PreservationMetadata,
}

impl AggLlmResponse {
    /// Extract assistant text content as a single string.
    pub fn assistant_text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| match b {
                super::conversation::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}
