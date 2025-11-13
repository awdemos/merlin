// src/providers/capabilities.rs
use serde::{Deserialize, Serialize};
use crate::api::DomainCategory;
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QualityScores {
    pub overall: f32,
    pub creativity: f32,
    pub reasoning: f32,
    pub code: f32,
    pub analytical: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelCapabilities {
    pub provider: String,
    pub model: String,
    pub cost_per_1k_tokens: f64,
    pub avg_latency_ms: u32,
    pub max_tokens: u32,
    pub context_window: usize,
    pub strengths: Vec<DomainCategory>,
    pub quality_scores: QualityScores,
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub supports_tools: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CapabilitiesConfig {
    #[serde(flatten)]
    pub models: HashMap<String, ModelCapabilities>,
}

pub struct CapabilityLoader {
    capabilities: HashMap<String, ModelCapabilities>,
}

impl CapabilityLoader {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }
    
    pub async fn load_from_file(&mut self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read capabilities file {}: {}", path, e))?;
        
        let config: CapabilitiesConfig = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse capabilities file {}: {}", path, e))?;
        
        for (key, capability) in config.models {
            self.capabilities.insert(key, capability);
        }
        
        Ok(())
    }
    
    pub async fn load_from_url(&mut self, url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client.get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch capabilities from {}: {}", url, e))?;
        
        let content = response.text().await
            .map_err(|e| anyhow::anyhow!("Failed to read response from {}: {}", url, e))?;
        
        let config: CapabilitiesConfig = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse capabilities from {}: {}", url, e))?;
        
        for (key, capability) in config.models {
            self.capabilities.insert(key, capability);
        }
        
        Ok(())
    }
    
    pub fn get_capabilities(&self, provider: &str, model: &str) -> Option<&ModelCapabilities> {
        let key = format!("{}.{}", provider, model);
        self.capabilities.get(&key)
    }
    
    pub fn get_capabilities_by_model(&self, model: &str) -> Option<&ModelCapabilities> {
        self.capabilities.get(model)
    }
    
    pub fn list_models(&self) -> Vec<String> {
        self.capabilities.keys().cloned().collect()
    }
    
    pub fn list_models_by_provider(&self, provider: &str) -> Vec<String> {
        self.capabilities
            .iter()
            .filter(|(_, cap)| cap.provider == provider)
            .map(|(key, _)| key.clone())
            .collect()
    }
    
    pub fn add_capability(&mut self, key: String, capability: ModelCapabilities) {
        self.capabilities.insert(key, capability);
    }
    
    pub fn get_default_capabilities() -> Self {
        let mut loader = Self::new();
        
        // Add some default capabilities for common models
        loader.add_capability(
            "openai.gpt-4-turbo".to_string(),
            ModelCapabilities {
                provider: "openai".to_string(),
                model: "gpt-4-turbo".to_string(),
                cost_per_1k_tokens: 0.03,
                avg_latency_ms: 2500,
                max_tokens: 4096,
                context_window: 128000,
                strengths: vec![
                    DomainCategory::Analytical,
                    DomainCategory::Technical,
                    DomainCategory::Mathematical,
                ],
                quality_scores: QualityScores {
                    overall: 0.95,
                    creativity: 0.85,
                    reasoning: 0.95,
                    code: 0.90,
                    analytical: 0.95,
                },
                supports_streaming: true,
                supports_function_calling: true,
                supports_vision: true,
                supports_tools: true,
            }
        );
        
        loader.add_capability(
            "anthropic.claude-3-opus-20240229".to_string(),
            ModelCapabilities {
                provider: "anthropic".to_string(),
                model: "claude-3-opus-20240229".to_string(),
                cost_per_1k_tokens: 0.075,
                avg_latency_ms: 2000,
                max_tokens: 4096,
                context_window: 200000,
                strengths: vec![
                    DomainCategory::Creative,
                    DomainCategory::Analytical,
                    DomainCategory::General,
                ],
                quality_scores: QualityScores {
                    overall: 0.92,
                    creativity: 0.95,
                    reasoning: 0.88,
                    code: 0.85,
                    analytical: 0.90,
                },
                supports_streaming: true,
                supports_function_calling: true,
                supports_vision: true,
                supports_tools: true,
            }
        );
        
        loader.add_capability(
            "mistral.mistral-large-latest".to_string(),
            ModelCapabilities {
                provider: "mistral".to_string(),
                model: "mistral-large-latest".to_string(),
                cost_per_1k_tokens: 0.008,
                avg_latency_ms: 1800,
                max_tokens: 8192,
                context_window: 32000,
                strengths: vec![
                    DomainCategory::Creative,
                    DomainCategory::CodeGeneration,
                    DomainCategory::Multilingual,
                ],
                quality_scores: QualityScores {
                    overall: 0.88,
                    creativity: 0.90,
                    reasoning: 0.85,
                    code: 0.82,
                    analytical: 0.86,
                },
                supports_streaming: true,
                supports_function_calling: true,
                supports_vision: false,
                supports_tools: true,
            }
        );
        
        loader
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_capability_loading() {
        let mut loader = CapabilityLoader::new();
        
        // Test default capabilities
        loader = CapabilityLoader::get_default_capabilities();
        
        let gpt4_caps = loader.get_capabilities("openai", "gpt-4-turbo");
        assert!(gpt4_caps.is_some());
        assert_eq!(gpt4_caps.unwrap().provider, "openai");
        
        let claude_caps = loader.get_capabilities("anthropic", "claude-3-opus-20240229");
        assert!(claude_caps.is_some());
        assert_eq!(claude_caps.unwrap().provider, "anthropic");
    }
    
    #[tokio::test]
    async fn test_capability_listing() {
        let loader = CapabilityLoader::get_default_capabilities();
        
        let models = loader.list_models();
        assert!(models.contains(&"openai.gpt-4-turbo".to_string()));
        assert!(models.contains(&"anthropic.claude-3-opus-20240229".to_string()));
        
        let openai_models = loader.list_models_by_provider("openai");
        assert!(openai_models.contains(&"openai.gpt-4-turbo".to_string()));
    }
}