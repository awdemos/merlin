//! Shared HTTP client abstractions for upstream LLM providers.

use crate::protocol::{LlmResponse, Request};
use async_trait::async_trait;
use std::fmt::Debug;

#[async_trait]
pub trait LlmClient: Send + Sync + Debug {
    fn id(&self) -> &str;
    fn wire_format(&self) -> crate::protocol::WireFormat;
    async fn execute(&self, request: Request) -> anyhow::Result<LlmResponse>;
}

pub mod translating;
pub use translating::TranslatingLlmClient;
