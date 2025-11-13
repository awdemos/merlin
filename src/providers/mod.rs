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

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, prompt: &str) -> anyhow::Result<String>;
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
