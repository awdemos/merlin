use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Correlation metadata for a request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
}

/// Per-target preservation metadata returned with the response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreservationMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_algorithm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_path: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Which wire format a target speaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireFormat {
    #[default]
    OpenAiChat,
    OpenAiResponses,
    AnthropicMessages,
    AnthropicRaw,
    Gemini,
    Ollama,
}

impl WireFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            WireFormat::OpenAiChat => "openai_chat",
            WireFormat::OpenAiResponses => "openai_responses",
            WireFormat::AnthropicMessages => "anthropic_messages",
            WireFormat::AnthropicRaw => "anthropic_raw",
            WireFormat::Gemini => "gemini",
            WireFormat::Ollama => "ollama",
        }
    }
}

impl std::str::FromStr for WireFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "openai_chat" => Ok(WireFormat::OpenAiChat),
            "openai_responses" => Ok(WireFormat::OpenAiResponses),
            "anthropic_messages" => Ok(WireFormat::AnthropicMessages),
            "anthropic_raw" => Ok(WireFormat::AnthropicRaw),
            "gemini" => Ok(WireFormat::Gemini),
            "ollama" => Ok(WireFormat::Ollama),
            _ => Err(format!("unknown wire format: {}", s)),
        }
    }
}

/// Reference to one of the configured targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetRef {
    pub name: String,
    pub client: String,
    pub model_id: String,
    pub format: WireFormat,
    pub base_url: String,
    #[serde(default)]
    pub timeout_ms: u64,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra_headers: HashMap<String, String>,
}

/// Full routed request envelope.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Request {
    pub llm_request: super::conversation::LlmRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra_body: HashMap<String, serde_json::Value>,
}

/// Outcome of routing: selected target + optional fallback chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingOutcome {
    pub target: TargetRef,
    pub fallback_chain: Vec<String>,
}
