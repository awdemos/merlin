use crate::LlmProvider;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(endpoint: String, model: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, prompt: &str) -> anyhow::Result<String> {
        let url = format!("{}/api/generate", self.endpoint);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": prompt,
                "stream": false
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Ollama request failed: {}",
                response.status()
            ));
        }

        let resp: Value = response.json().await?;

        if let Some(response_text) = resp["response"].as_str() {
            Ok(response_text.to_string())
        } else {
            Err(anyhow::anyhow!("Invalid response format from Ollama"))
        }
    }

    fn name(&self) -> &'static str {
        "Ollama"
    }
}
