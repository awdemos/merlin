use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    pub fn to_wire(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "system" | "developer" => Some(Role::System),
            "user" => Some(Role::User),
            "assistant" => Some(Role::Assistant),
            "tool" => Some(Role::Tool),
            _ => None,
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Role::from_wire(s).ok_or_else(|| format!("unknown role: {}", s))
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_wire())
    }
}

/// A content block inside a message or instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
        mime_type: String,
    },
    ToolResult {
        call_id: String,
        output: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { data: String },
    Url { url: String },
}

impl ImageSource {
    pub fn to_url(&self) -> String {
        match self {
            ImageSource::Base64 { data } => format!("data:image;base64,{}", data),
            ImageSource::Url { url } => url.clone(),
        }
    }
}

/// A system / developer instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstructionBlock {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

/// A single conversation turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<super::tools::ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn text(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![ContentBlock::Text { text: text.into() }],
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

/// Generation controls.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SamplingParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

/// Output constraints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

/// The core LLM request in provider-neutral form.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub instructions: Vec<InstructionBlock>,
    pub messages: Vec<Message>,
    pub tools: Vec<super::tools::ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<super::tools::ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputParams>,
    pub stream: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl LlmRequest {
    /// Return all user-visible text concatenated (for feature extraction / embedding).
    pub fn prompt_text(&self) -> String {
        let mut out = String::new();
        for inst in &self.instructions {
            for block in &inst.content {
                if let ContentBlock::Text { text } = block {
                    out.push_str(text);
                    out.push(' ');
                }
            }
        }
        for msg in &self.messages {
            for block in &msg.content {
                if let ContentBlock::Text { text } = block {
                    out.push_str(text);
                    out.push(' ');
                }
            }
        }
        out.trim().to_string()
    }
}
