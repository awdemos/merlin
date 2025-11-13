// src/providers/bedrock.rs
use crate::providers::{LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Result;
use chrono::{DateTime, Utc};
use base64::{Engine as _, engine::general_purpose};

pub struct BedrockProvider {
    client: Client,
    access_key: String,
    secret_key: String,
    region: String,
    model: String,
    capabilities: Option<ModelCapabilities>,
}

impl BedrockProvider {
    pub fn new(access_key: String, secret_key: String, region: String, model: String) -> Self {
        Self {
            client: Client::new(),
            access_key,
            secret_key,
            region,
            model,
            capabilities: None,
        }
    }

    fn create_aws_signature(&self, method: &str, service: &str, region: &str, host: &str, path: &str, payload: &str, datetime: &DateTime<Utc>) -> String {
        // Simplified AWS Signature Version 4
        // In production, use AWS SDK for proper signing
        let datestamp = datetime.format("%Y%m%d").to_string();
        let amz_date = datetime.format("%Y%m%dT%H%M%SZ").to_string();
        
        // This is a simplified version - production should use proper AWS SDK
        format!("AWS4-HMAC-SHA256 Credential={}/{}/{}/{}/aws4_request, SignedHeaders=host;x-amz-date, Signature=placeholder",
                self.access_key, datestamp, region, service)
    }
}

#[async_trait]
impl LlmProvider for BedrockProvider {
    async fn chat(&self, prompt: &str) -> Result<String> {
        let host = format!("bedrock-runtime.{}.amazonaws.com", self.region);
        let url = format!("https://{}/model/{}/invoke", host, self.model);
        
        let datetime = Utc::now();
        let payload = json!({
            "inputText": prompt,
            "textGenerationConfig": {
                "maxTokenCount": 4096,
                "temperature": 0.7,
                "topP": 0.9
            }
        }).to_string();

        let authorization = self.create_aws_signature("POST", "bedrock", &self.region, &host, "/model/{}", &payload, &datetime);

        let response = self
            .client
            .post(&url)
            .header("Authorization", authorization)
            .header("Content-Type", "application/json")
            .header("X-Amz-Date", datetime.format("%Y%m%dT%H%M%SZ").to_string())
            .header("Host", &host)
            .body(payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Bedrock request failed: {}",
                response.status()
            ));
        }

        let resp: Value = response.json().await?;

        if let Some(output) = resp["results"][0]["outputText"].as_str() {
            Ok(output.to_string())
        } else {
            Err(anyhow::anyhow!("Invalid response format from Bedrock"))
        }
    }

    fn name(&self) -> &'static str {
        "AWS Bedrock"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn capabilities(&self) -> Option<&ModelCapabilities> {
        self.capabilities.as_ref()
    }
}