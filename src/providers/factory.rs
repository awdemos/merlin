// src/providers/factory.rs
use async_trait::async_trait;
use crate::providers::{LlmProvider, config::ProviderConfig};
use anyhow::Result;

#[async_trait]
pub trait ProviderFactory: Send + Sync {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>>;
    fn name(&self) -> &str;
    fn supported_models(&self) -> Vec<String>;
    fn validate_config(&self, config: &ProviderConfig) -> Result<()> {
        config.validate()
    }
}

pub struct OpenAiFactory;

#[async_trait]
impl ProviderFactory for OpenAiFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::OpenAiProvider::new(
            config.get_api_key()?,
            config.default_model.clone(),
            config.base_url.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "openai"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "gpt-4-turbo".to_string(),
            "gpt-4".to_string(),
            "gpt-3.5-turbo".to_string(),
            "gpt-3.5-turbo-16k".to_string(),
        ]
    }
}

pub struct AnthropicFactory;

#[async_trait]
impl ProviderFactory for AnthropicFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::AnthropicProvider::new(
            config.get_api_key()?,
            config.default_model.clone(),
            config.base_url.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "anthropic"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
            "claude-2.1".to_string(),
            "claude-2.0".to_string(),
        ]
    }
}

pub struct MistralFactory;

#[async_trait]
impl ProviderFactory for MistralFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::MistralProvider::new(
            config.get_api_key()?,
            config.default_model.clone(),
            config.base_url.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "mistral"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "mistral-large-latest".to_string(),
            "mistral-medium-latest".to_string(),
            "mistral-small-latest".to_string(),
            "codestral-latest".to_string(),
        ]
    }
}

pub struct GeminiFactory;

#[async_trait]
impl ProviderFactory for GeminiFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::GeminiProvider::new(
            config.get_api_key()?,
            config.default_model.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "gemini"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "gemini-1.5-pro-latest".to_string(),
            "gemini-1.5-flash-latest".to_string(),
            "gemini-1.0-pro-latest".to_string(),
        ]
    }
}

pub struct GroqFactory;

#[async_trait]
impl ProviderFactory for GroqFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::GroqProvider::new(
            config.get_api_key()?,
            config.default_model.clone(),
            config.base_url.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "groq"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "llama-3.1-70b-versatile".to_string(),
            "llama-3.1-8b-instant".to_string(),
            "mixtral-8x7b-32768".to_string(),
            "gemma-7b-it".to_string(),
        ]
    }
}

pub struct OllamaFactory;

#[async_trait]
impl ProviderFactory for OllamaFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::OllamaProvider::new(
            config.base_url.clone(),
            config.default_model.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "ollama"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "llama3.1".to_string(),
            "llama2".to_string(),
            "mistral".to_string(),
            "codellama".to_string(),
            "phi3".to_string(),
            "qwen2".to_string(),
        ]
    }
}

pub struct GrokFactory;

#[async_trait]
impl ProviderFactory for GrokFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::GrokProvider::new(
            config.get_api_key()?,
            config.default_model.clone(),
            config.base_url.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "grok"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "grok-beta".to_string(),
            "grok-2".to_string(),
        ]
    }
}

pub struct ZAIFactory;

#[async_trait]
impl ProviderFactory for ZAIFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::ZAIProvider::new(
            config.get_api_key()?,
            config.default_model.clone(),
            config.base_url.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "zai"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "zai-7b".to_string(),
            "zai-13b".to_string(),
            "zai-34b".to_string(),
        ]
    }
}

pub struct MoonshotFactory;

#[async_trait]
impl ProviderFactory for MoonshotFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::MoonshotProvider::new(
            config.get_api_key()?,
            config.default_model.clone(),
            config.base_url.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "moonshot"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "moonshot-v1-8k".to_string(),
            "moonshot-v1-32k".to_string(),
            "moonshot-v1-128k".to_string(),
        ]
    }
}

pub struct BedrockFactory;

#[async_trait]
impl ProviderFactory for BedrockFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        let region = config.custom_params.get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("us-east-1")
            .to_string();
        
        Ok(Box::new(crate::providers::BedrockProvider::new(
            config.get_api_key()?,
            config.get_api_key()?, // Using same field for secret key in this simplified version
            region,
            config.default_model.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "bedrock"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "anthropic.claude-3-opus-20240229-v1:0".to_string(),
            "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
            "meta.llama3-70b-instruct-v1:0".to_string(),
            "mistral.mistral-large-2402-v1:0".to_string(),
        ]
    }
}

pub struct LambdaLabsFactory;

#[async_trait]
impl ProviderFactory for LambdaLabsFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        Ok(Box::new(crate::providers::LambdaLabsProvider::new(
            config.get_api_key()?,
            config.default_model.clone(),
            config.base_url.clone(),
        )))
    }
    
    fn name(&self) -> &str {
        "lambdalabs"
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "hermes-2-pro-llama-3-8b".to_string(),
            "hermes-2-theta-llama-3-70b".to_string(),
            "code-llama-34b-instruct".to_string(),
        ]
    }
}