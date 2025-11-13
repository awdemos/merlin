// src/providers/mistral.rs
use crate::providers::{LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Result;

pub struct MistralProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    capabilities: Option<ModelCapabilities>,
}

impl MistralProvider {
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
impl LlmProvider for MistralProvider {
    async fn chat(&self, prompt: &str) -> Result<String> {
        let response = self.client
            .post(&format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": self.model,
                "messages": [{"role": "user", "content": prompt}],
                "max_tokens": 4096,
                "temperature": 0.7
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Mistral API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let resp: Value = response.json().await?;
        
        match resp["choices"].as_array() {
            Some(choices) => {
                if let Some(first_choice) = choices.first() {
                    if let Some(message) = first_choice["message"].as_object() {
                        if let Some(content) = message["content"].as_str() {
                            return Ok(content.to_string());
                        }
                    }
                }
            }
            None => {}
        }

        Err(anyhow::anyhow!("Invalid response format from Mistral API"))
    }

    fn name(&self) -> &'static str {
        "Mistral"
    }
    
    fn model(&self) -> &str {
        &self.model
    }
    
    fn capabilities(&self) -> Option<&ModelCapabilities> {
        self.capabilities.as_ref()
    }
}