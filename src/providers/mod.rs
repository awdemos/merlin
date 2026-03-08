//! LLM provider implementations for Merlin.
//!
//! This module contains provider implementations for various LLM services
//! including OpenAI and Ollama. All providers implement the [`LlmProvider`] trait.

pub mod ollama;
pub mod openai;
pub mod anthropic;
pub mod mistral;
pub mod gemini;
pub mod groq;
pub mod grok;
pub mod zai;
pub mod moonshot;
pub mod bedrock;
pub mod lambdalabs;
pub mod config;
pub mod factory;
pub mod registry;
pub mod capabilities;

pub use config::{ProviderConfig, MerlinConfig};
pub use factory::ProviderFactory;
pub use registry::ProviderRegistry;
pub use capabilities::{ModelCapabilities, CapabilityLoader, QualityScores};

use async_trait::async_trait;

/// Trait for LLM provider implementations.
///
/// All LLM providers must implement this trait to be used with the Merlin router.
/// The trait provides a common interface for sending chat completions and
/// identifying the provider.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Sends a chat completion request and returns the generated response.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The input text to send to the LLM
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response is invalid.
    async fn chat(&self, prompt: &str) -> anyhow::Result<String>;
    
    /// Returns the name of this provider for logging and metrics.
    fn name(&self) -> &'static str;
    fn model(&self) -> &str;
    fn capabilities(&self) -> Option<&ModelCapabilities>;
}

pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;
pub use mistral::MistralProvider;
pub use gemini::GeminiProvider;
pub use groq::GroqProvider;
pub use grok::GrokProvider;
pub use zai::ZAIProvider;
pub use moonshot::MoonshotProvider;
pub use bedrock::BedrockProvider;
pub use lambdalabs::LambdaLabsProvider;
