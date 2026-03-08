//! LLM provider implementations for Merlin.
//!
//! This module contains provider implementations for various LLM services
//! including OpenAI and Ollama. All providers implement the [`LlmProvider`] trait.

pub mod ollama;
pub mod openai;

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
}

pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
