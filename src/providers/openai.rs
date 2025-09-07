// src/providers/openai.rs
use crate::LlmProvider;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: String,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, prompt: &str) -> anyhow::Result<String> {
        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap()
            .into())
    }

    fn name(&self) -> &'static str {
        "OpenAI" // Return provider name
    }
}
