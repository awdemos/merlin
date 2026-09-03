//! Translation codecs between wire formats and the provider-neutral protocol.

use crate::protocol::{LlmResponse, Request};

pub mod anthropic;
pub mod openai;
pub mod openai_decode;

pub trait WireCodec: Send + Sync + std::fmt::Debug {
    fn wire_format(&self) -> crate::protocol::WireFormat;

    /// Decode an inbound wire-format JSON body into a provider-neutral Request.
    fn decode_request(&self, body: serde_json::Value) -> anyhow::Result<Request>;

    /// Encode a provider-neutral Request for the upstream provider.
    fn encode_request(&self, request: &Request) -> anyhow::Result<serde_json::Value>;

    /// Decode a non-streaming upstream response into a provider-neutral response.
    fn decode_response(
        &self,
        status: reqwest::StatusCode,
        body: serde_json::Value,
    ) -> anyhow::Result<LlmResponse>;

    /// Decode one Server-Sent Events line into a stream event, if any.
    fn decode_sse_chunk(
        &self,
        chunk: &str,
    ) -> anyhow::Result<Option<crate::protocol::LlmResponseStreamEvent>>;
}

pub fn codec_for(format: crate::protocol::WireFormat) -> Box<dyn WireCodec> {
    match format {
        crate::protocol::WireFormat::AnthropicMessages => {
            Box::new(anthropic::AnthropicMessagesCodec)
        }
        _ => Box::new(openai::OpenAiChatCodec),
    }
}
