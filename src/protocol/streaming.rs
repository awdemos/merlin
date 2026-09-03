use serde::{Deserialize, Serialize};

/// One chunk from an LLM response stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum LlmResponseChunk {
    ContentDelta {
        block: super::conversation::ContentBlock,
    },
    ToolCallDelta {
        calls: Vec<super::tools::ToolCall>,
    },
    Usage {
        usage: super::response::Usage,
    },
    /// Stream still in progress but no content/tool in this chunk.
    Progress,
    /// Stream completed.
    Done,
}

/// A single-consumption stream of response chunks.
pub struct LlmResponseStream {
    pub inner: BoxStream,
}

impl std::fmt::Debug for LlmResponseStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmResponseStream").finish_non_exhaustive()
    }
}

/// Type alias for the boxed stream of chunks.
pub type BoxStream = std::pin::Pin<Box<dyn futures::Stream<Item = LlmResponseStreamEvent> + Send>>;

/// An event from a response stream, possibly carrying a provider-specific replay value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponseStreamEvent {
    pub chunk: LlmResponseChunk,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<super::response::Usage>,
    #[serde(skip)]
    pub provider_event: Option<serde_json::Value>,
}

impl LlmResponseStreamEvent {
    pub fn new(chunk: LlmResponseChunk) -> Self {
        Self {
            chunk,
            usage: None,
            provider_event: None,
        }
    }

    pub fn with_provider_event(mut self, value: serde_json::Value) -> Self {
        self.provider_event = Some(value);
        self
    }
}

/// Either a completed response or a stream.
#[derive(Debug)]
pub enum LlmResponse {
    Aggregate(Box<super::response::AggLlmResponse>),
    Stream(LlmResponseStream),
}
