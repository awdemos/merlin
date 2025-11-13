// src/providers/gemini.rs
use crate::providers::{LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Result;

pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model: String,
    capabilities: Option<ModelCapabilities>,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            capabilities: None,
        }
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn chat(&self, prompt: &str) -> Result<String> {
        let response = self.client
            .post(&format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", 
                self.model, self.api_key
            ))
            .json(&json!({
                "contents": [{"parts": [{"text": prompt}]}],
                "generationConfig": {
                    "temperature": 0.7,
                    "maxOutputTokens": 2048,
                    "candidateCount": 1
                }
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Gemini API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let resp: Value = response.json().await?;
        
        match resp["candidates"].as_array() {
            Some(candidates) => {
                if let Some(first_candidate) = candidates.first() {
                    if let Some(content) = first_candidate["content"].as_object() {
                        if let Some(parts) = content["parts"].as_array() {
                            if let Some(first_part) = parts.first() {
                                if let Some(text) = first_part["text"].as_str() {
                                    return Ok(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
            None => {}
        }

        Err(anyhow::anyhow!("Invalid response format from Gemini API"))
    }

    fn name(&self) -> &'static str {
        "Gemini"
    }
    
    fn model(&self) -> &str {
        &self.model
    }
    
    fn capabilities(&self) -> Option<&ModelCapabilities> {
        self.capabilities.as_ref()
    }
}