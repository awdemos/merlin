// src/providers/lambdalabs.rs
use crate::providers::{LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Result;

pub struct LambdaLabsProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    capabilities: Option<ModelCapabilities>,
}

impl LambdaLabsProvider {
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
impl LlmProvider for LambdaLabsProvider {
    async fn chat(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "max_tokens": 4096,
                "temperature": 0.7
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Lambda Labs request failed: {}",
                response.status()
            ));
        }

        let resp: Value = response.json().await?;

        if let Some(content) = resp["choices"][0]["message"]["content"].as_str() {
            Ok(content.to_string())
        } else {
            Err(anyhow::anyhow!("Invalid response format from Lambda Labs"))
        }
    }

    fn name(&self) -> &'static str {
        "Lambda Labs"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn capabilities(&self) -> Option<&ModelCapabilities> {
        self.capabilities.as_ref()
    }
}