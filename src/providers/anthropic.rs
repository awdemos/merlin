// src/providers/anthropic.rs
use crate::providers::{LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Result;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    capabilities: Option<ModelCapabilities>,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url,
            capabilities: None,
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, prompt: &str) -> Result<String> {
        let response = self.client
            .post(&format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&json!({
                "model": self.model,
                "max_tokens": 4096,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Anthropic API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let resp: Value = response.json().await?;
        
        match resp["content"].as_array() {
            Some(content_array) => {
                if let Some(first_content) = content_array.first() {
                    if let Some(text) = first_content["text"].as_str() {
                        return Ok(text.to_string());
                    }
                }
            }
            None => {}
        }

        Err(anyhow::anyhow!("Invalid response format from Anthropic API"))
    }

    fn name(&self) -> &'static str {
        "Anthropic"
    }
    
    fn model(&self) -> &str {
        &self.model
    }
    
    fn capabilities(&self) -> Option<&ModelCapabilities> {
        self.capabilities.as_ref()
    }
}