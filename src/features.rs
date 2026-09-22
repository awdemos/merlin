//! Client for the feature-extraction sidecar (a Python HTTP service wrapping
//! the `laya` decision engine).
//!
//! The sidecar classifies each prompt (domain, difficulty, needs_tools,
//! is_sensitive) and provides guardrail verdicts (jailbreak, prompt_injection).
//! Every call is fail-open: on any error (connection refused, timeout,
//! non-2xx, malformed JSON) a `tracing::warn!` is emitted and `None` is
//! returned so routing can fall back to degraded local behavior.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Domains in fixed one-hot order. `unknown` maps to all zeros.
pub const DOMAINS: [&str; PromptFeatures::DOMAIN_DIM] = [
    "code",
    "math_or_logic",
    "writing",
    "factual_lookup",
    "data_analysis",
    "chitchat",
];

/// Classified prompt features returned by the sidecar `/features` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptFeatures {
    pub domain: String,
    pub domain_confidence: f64,
    pub difficulty: f64,
    pub difficulty_confidence: f64,
    pub needs_tools: f64,
    pub is_sensitive: f64,
}

impl PromptFeatures {
    /// Number of domain one-hot dimensions in [`PromptFeatures::vector`].
    pub const DOMAIN_DIM: usize = 6;

    /// Context vector for the contextual bandit: bias, difficulty,
    /// needs_tools, is_sensitive, then a 6-dim domain one-hot (10 dims).
    /// An `unknown` domain yields all-zero one-hot.
    pub fn vector(&self) -> Vec<f64> {
        let mut v = vec![1.0, self.difficulty, self.needs_tools, self.is_sensitive];
        for d in DOMAINS {
            v.push(if self.domain == d { 1.0 } else { 0.0 });
        }
        v
    }

    /// Degraded feature set for when the sidecar is unavailable: unknown
    /// domain, difficulty estimated from prompt length (clamped to [0, 1]),
    /// everything else zero. Keeps a useful signal for the bandit.
    pub fn legacy(prompt_len: usize) -> Self {
        Self {
            domain: "unknown".to_string(),
            domain_confidence: 0.0,
            difficulty: (prompt_len as f64 / 400.0).clamp(0.0, 1.0),
            difficulty_confidence: 0.0,
            needs_tools: 0.0,
            is_sensitive: 0.0,
        }
    }
}

/// Guardrail verdict returned by the sidecar `/guard` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardVerdict {
    pub jailbreak: f64,
    pub prompt_injection: f64,
    pub sensitive_data: f64,
    pub harm_severity: f64,
    pub topic: String,
}

impl GuardVerdict {
    /// Reason to block the prompt, if any: `jailbreak` is checked before
    /// `prompt_injection`; a value at or above its threshold blocks.
    pub fn block_reason(
        &self,
        jailbreak_threshold: f64,
        injection_threshold: f64,
    ) -> Option<String> {
        if self.jailbreak >= jailbreak_threshold {
            Some("jailbreak".to_string())
        } else if self.prompt_injection >= injection_threshold {
            Some("prompt_injection".to_string())
        } else {
            None
        }
    }
}

/// HTTP client for the feature-extraction sidecar with a small in-memory
/// cache for `/features` responses (keyed by full prompt text).
#[derive(Debug)]
pub struct FeatureClient {
    client: reqwest::Client,
    base_url: String,
    timeout: Duration,
    cache: Mutex<HashMap<String, (Instant, PromptFeatures)>>,
    cache_entries: usize,
    cache_ttl: Duration,
}

impl FeatureClient {
    pub fn new(
        base_url: String,
        timeout_ms: u64,
        cache_entries: usize,
        cache_ttl_secs: u64,
    ) -> Self {
        let timeout = Duration::from_millis(timeout_ms);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "failed to build feature client with timeout {:?}: {}; using default client",
                    timeout,
                    e
                );
                reqwest::Client::new()
            });
        Self {
            client,
            base_url,
            timeout,
            cache: Mutex::new(HashMap::new()),
            cache_entries,
            cache_ttl: Duration::from_secs(cache_ttl_secs),
        }
    }

    /// Classify a prompt. Returns `None` on any sidecar error (fail-open).
    pub async fn features(&self, prompt: &str) -> Option<PromptFeatures> {
        if let Some((fetched, features)) = self.cache.lock().unwrap().get(prompt) {
            if fetched.elapsed() <= self.cache_ttl {
                return Some(features.clone());
            }
        }

        let url = format!("{}/features", self.base_url.trim_end_matches('/'));
        let response = match self
            .client
            .post(&url)
            .timeout(self.timeout)
            .json(&serde_json::json!({ "prompt": prompt }))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("feature sidecar request to {} failed: {}", url, e);
                return None;
            }
        };
        if !response.status().is_success() {
            tracing::warn!(
                "feature sidecar returned status {} for /features",
                response.status()
            );
            return None;
        }
        let features = match response.json::<PromptFeatures>().await {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("failed to parse /features response: {}", e);
                return None;
            }
        };

        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= self.cache_entries {
            // Evict expired entries first; if still full, clear everything.
            cache.retain(|_, (fetched, _)| fetched.elapsed() <= self.cache_ttl);
            if cache.len() >= self.cache_entries {
                cache.clear();
            }
        }
        cache.insert(prompt.to_string(), (Instant::now(), features.clone()));
        Some(features)
    }

    /// Guardrail verdict for a prompt. Not cached. Returns `None` on any
    /// sidecar error (fail-open; the caller allows the request through).
    pub async fn guard(&self, prompt: &str) -> Option<GuardVerdict> {
        let url = format!("{}/guard", self.base_url.trim_end_matches('/'));
        let response = match self
            .client
            .post(&url)
            .timeout(self.timeout)
            .json(&serde_json::json!({ "prompt": prompt }))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("guard sidecar request to {} failed: {}", url, e);
                return None;
            }
        };
        if !response.status().is_success() {
            tracing::warn!(
                "guard sidecar returned status {} for /guard",
                response.status()
            );
            return None;
        }
        match response.json::<GuardVerdict>().await {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!("failed to parse /guard response: {}", e);
                None
            }
        }
    }
}
