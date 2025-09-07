pub mod ollama;
pub mod openai;

use async_trait::async_trait;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, prompt: &str) -> anyhow::Result<String>;
    fn name(&self) -> &'static str;
}

pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
