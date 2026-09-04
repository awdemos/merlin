//! Translating HTTP client: shared reqwest connection pool + retries.

use crate::clients::LlmClient;
use crate::config::ClientConfig;
use crate::protocol::{LlmResponse, LlmResponseStreamEvent, Request, WireFormat};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use std::sync::Arc;
use std::time::Duration;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::RetryIf;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub struct TranslatingLlmClient {
    id: String,
    format: WireFormat,
    base_url: String,
    api_key_env: Option<String>,
    client: reqwest::Client,
    max_retries: u32,
}

impl TranslatingLlmClient {
    pub fn new(id: String, config: &ClientConfig) -> anyhow::Result<Self> {
        let format = match config.format.as_str() {
            "openai_chat" => WireFormat::OpenAiChat,
            "anthropic_messages" => WireFormat::AnthropicMessages,
            other => anyhow::bail!("unknown client format: {}", other),
        };

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        for (k, v) in &config.extra_headers {
            let name = HeaderName::from_bytes(k.as_bytes())?;
            let value = HeaderValue::from_str(v)?;
            headers.insert(name, value);
        }

        let client = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(32)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            id,
            format,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key_env: config.api_key_env.clone(),
            client,
            max_retries: config.max_retries,
        })
    }

    fn api_key(&self) -> Option<String> {
        self.api_key_env
            .as_ref()
            .and_then(|env| std::env::var(env).ok())
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn path(&self) -> &'static str {
        match self.format {
            WireFormat::AnthropicMessages => "/messages",
            _ => "/chat/completions",
        }
    }
}

#[async_trait]
impl LlmClient for TranslatingLlmClient {
    fn id(&self) -> &str {
        &self.id
    }

    fn wire_format(&self) -> WireFormat {
        self.format
    }

    async fn execute(&self, request: Request) -> anyhow::Result<LlmResponse> {
        let codec = Arc::new(crate::translation::codec_for(self.format));
        let body = codec.encode_request(&request)?;
        let stream = request.llm_request.stream;

        let send = || {
            let client = self.client.clone();
            let url = self.url(self.path());
            let body = body.clone();
            let api_key = self.api_key();
            async move {
                let req = client.post(url).json(&body);
                let req = if let Some(key) = api_key {
                    req.bearer_auth(key)
                } else {
                    req
                };
                req.send().await
            }
        };

        let response = if self.max_retries > 0 {
            RetryIf::spawn(
                ExponentialBackoff::from_millis(100).take(self.max_retries as usize),
                send,
                |e: &reqwest::Error| e.is_timeout() || e.is_connect(),
            )
            .await?
        } else {
            send().await?
        };

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("upstream error {}: {}", status, text));
        }

        if stream {
            let codec = codec.clone();
            let events = response.bytes_stream().flat_map(
                move |chunk: Result<bytes::Bytes, reqwest::Error>| {
                    let codec = codec.clone();
                    let mut events: Vec<LlmResponseStreamEvent> = Vec::new();
                    if let Ok(bytes) = chunk {
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            if let Ok(Some(event)) = codec.decode_sse_chunk(line) {
                                events.push(event);
                            }
                        }
                    }
                    futures::stream::iter(events)
                },
            );

            Ok(LlmResponse::Stream(crate::protocol::LlmResponseStream {
                inner: Box::pin(events),
            }))
        } else {
            let body: serde_json::Value = response.json().await?;
            codec.decode_response(status, body)
        }
    }
}
