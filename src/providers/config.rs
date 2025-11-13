// src/providers/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub base_url: String,
    pub models: Vec<String>,
    pub default_model: String,
    #[serde(default)]
    pub custom_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProvidersConfig {
    pub registry: Vec<String>,
    #[serde(flatten)]
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MerlinConfig {
    pub providers: ProvidersConfig,
    pub server: ServerConfig,
    pub routing: RoutingConfig,
    pub metrics: MetricsConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoutingConfig {
    pub policy: String,
    #[serde(default = "default_epsilon")]
    pub epsilon: f64,
    pub capabilities_file: Option<String>,
}

fn default_epsilon() -> f64 {
    0.15
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    pub redis_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelemetryConfig {
    pub prometheus_port: u16,
    pub jaeger_endpoint: String,
}

impl ProviderConfig {
    pub fn get_api_key(&self) -> Result<String> {
        match &self.api_key {
            Some(key) if key.starts_with("${") && key.ends_with('}') => {
                let env_var = &key[2..key.len()-1];
                std::env::var(env_var)
                    .map_err(|_| anyhow::anyhow!("Environment variable {} not found", env_var))
            }
            Some(key) => Ok(key.clone()),
            None => Err(anyhow::anyhow!("No API key configured")),
        }
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.enabled && self.api_key.is_some() {
            self.get_api_key()?;
        }
        
        if self.models.is_empty() {
            return Err(anyhow::anyhow!("No models configured for provider"));
        }
        
        if !self.models.contains(&self.default_model) {
            return Err(anyhow::anyhow!("Default model not in models list"));
        }
        
        Ok(())
    }
}

impl MerlinConfig {
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file {}: {}", path, e))?;
        
        toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file {}: {}", path, e))
    }
    
    pub fn load_from_env() -> Result<Self> {
        let config_path = std::env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "merlin.toml".to_string());
        Self::load_from_file(&config_path)
    }
    
    pub fn validate(&self) -> Result<()> {
        for (name, config) in &self.providers.providers {
            config.validate()
                .map_err(|e| anyhow::anyhow!("Invalid config for provider {}: {}", name, e))?;
        }
        Ok(())
    }
}

impl Default for MerlinConfig {
    fn default() -> Self {
        Self {
            providers: ProvidersConfig {
                registry: vec![
                    "openai".to_string(),
                    "anthropic".to_string(),
                    "mistral".to_string(),
                    "gemini".to_string(),
                    "grok".to_string(),
                    "groq".to_string(),
                    "zai".to_string(),
                    "moonshot".to_string(),
                    "bedrock".to_string(),
                    "lambdalabs".to_string(),
                    "ollama".to_string(),
                ],
                providers: HashMap::new(),
            },
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 7777,
            },
            routing: RoutingConfig {
                policy: "epsilon_greedy".to_string(),
                epsilon: 0.15,
                capabilities_file: Some("capabilities.toml".to_string()),
            },
            metrics: MetricsConfig {
                redis_url: "redis://127.0.0.1:6379".to_string(),
            },
            telemetry: TelemetryConfig {
                prometheus_port: 9090,
                jaeger_endpoint: "http://localhost:14268/api/traces".to_string(),
            },
        }
    }
}