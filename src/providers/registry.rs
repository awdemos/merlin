// src/providers/registry.rs
use crate::providers::{config::ProviderConfig, factory::{ProviderFactory, OpenAiFactory, AnthropicFactory, MistralFactory, GeminiFactory, GroqFactory, OllamaFactory, GrokFactory, ZAIFactory, MoonshotFactory, BedrockFactory, LambdaLabsFactory}};
use anyhow::Result;
use std::collections::HashMap;

pub struct ProviderRegistry {
    factories: HashMap<String, Box<dyn ProviderFactory>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register_default_factories(&mut self) {
        // Register all available providers
        self.register_factory(Box::new(OpenAiFactory));
        self.register_factory(Box::new(AnthropicFactory));
        self.register_factory(Box::new(MistralFactory));
        self.register_factory(Box::new(GeminiFactory));
        self.register_factory(Box::new(GroqFactory));
        self.register_factory(Box::new(OllamaFactory));
        self.register_factory(Box::new(GrokFactory));
        self.register_factory(Box::new(ZAIFactory));
        self.register_factory(Box::new(MoonshotFactory));
        self.register_factory(Box::new(BedrockFactory));
        self.register_factory(Box::new(LambdaLabsFactory));
    }
    
    pub fn register_factory(&mut self, factory: Box<dyn ProviderFactory>) {
        let name = factory.name().to_string();
        self.factories.insert(name, factory);
    }
    
    pub fn create_provider(&self, name: &str, config: &ProviderConfig) -> Result<Box<dyn crate::providers::LlmProvider>> {
        match self.factories.get(name) {
            Some(factory) => factory.create(config),
            None => Err(anyhow::anyhow!("Unknown provider: {}", name)),
        }
    }
    
    pub fn get_supported_models(&self, name: &str) -> Result<Vec<String>> {
        match self.factories.get(name) {
            Some(factory) => Ok(factory.supported_models()),
            None => Err(anyhow::anyhow!("Unknown provider: {}", name)),
        }
    }
    
    pub fn list_providers(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }
    
    pub fn validate_provider_config(&self, name: &str, config: &ProviderConfig) -> Result<()> {
        match self.factories.get(name) {
            Some(factory) => factory.validate_config(config),
            None => Err(anyhow::anyhow!("Unknown provider: {}", name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::config::{ProviderConfig, MerlinConfig};
    
    #[tokio::test]
    async fn test_provider_registry() {
        let mut registry = ProviderRegistry::new();
        registry.register_default_factories();
        
        // Test listing providers
        let providers = registry.list_providers();
        assert!(providers.contains(&"openai".to_string()));
        assert!(providers.contains(&"anthropic".to_string()));
        assert!(providers.contains(&"mistral".to_string()));
        
        // Test getting supported models
        let openai_models = registry.get_supported_models("openai").unwrap();
        assert!(openai_models.contains(&"gpt-4-turbo".to_string()));
        
        // Test unknown provider
        assert!(registry.get_supported_models("unknown").is_err());
    }
    
    #[tokio::test]
    async fn test_provider_creation() {
        let mut registry = ProviderRegistry::new();
        registry.register_default_factories();
        
        let config = ProviderConfig {
            enabled: true,
            api_key: Some("test-key".to_string()),
            base_url: "https://api.openai.com/v1".to_string(),
            models: vec!["gpt-4-turbo".to_string()],
            default_model: "gpt-4-turbo".to_string(),
            custom_params: std::collections::HashMap::new(),
        };
        
        // This would fail without proper API key, but tests the factory pattern
        assert!(registry.create_provider("openai", &config).is_ok());
        assert!(registry.create_provider("unknown", &config).is_err());
    }
}