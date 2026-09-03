use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Top-level deployment configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MerlinConfig {
    pub server: ServerConfig,
    pub clients: HashMap<String, ClientConfig>,
    pub targets: HashMap<String, TargetConfig>,
    pub routes: HashMap<String, RouteConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 7777,
        }
    }
}

/// Upstream LLM client configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientConfig {
    pub format: String,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default)]
    pub forward_auth: bool,
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_max_retries() -> u32 {
    2
}

fn default_timeout_ms() -> u64 {
    120000
}

impl ClientConfig {
    pub fn format(&self) -> crate::protocol::WireFormat {
        crate::protocol::WireFormat::from_str(&self.format)
            .unwrap_or(crate::protocol::WireFormat::OpenAiChat)
    }
}

/// A target is one upstream model ID plus the client used to call it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TargetConfig {
    #[serde(default)]
    pub name: String,
    pub client: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_body: Option<crate::protocol::tools::ExtraBody>,
}

impl TargetConfig {
    pub fn format(&self) -> crate::protocol::WireFormat {
        crate::protocol::WireFormat::from_str(&self.client)
            .unwrap_or(crate::protocol::WireFormat::OpenAiChat)
    }
}

/// A route is a client-visible model ID plus a routing algorithm.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteConfig {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    pub targets: Vec<String>,
    pub algorithm: AlgorithmConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlgorithmConfig {
    Passthrough {
        target: String,
    },
    Random {
        targets: Vec<String>,
        weights: Option<Vec<f64>>,
        seed: Option<u64>,
    },
    EpsilonGreedy {
        targets: Vec<String>,
        epsilon: f64,
    },
    Thompson {
        targets: Vec<String>,
    },
    Ucb {
        targets: Vec<String>,
        confidence_level: Option<f64>,
    },
    Contextual {
        targets: Vec<String>,
        learning_rate: Option<f64>,
        exploration_rate: Option<f64>,
    },
    LlmClassifier {
        classifier_target: String,
        strong_target: String,
        weak_target: String,
        #[serde(default = "default_threshold")]
        base_threshold: f64,
    },
}

fn default_threshold() -> f64 {
    0.5
}

impl MerlinConfig {
    pub fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file {}: {}", path, e))?;
        let mut config: Self = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file {}: {}", path, e))?;
        for (name, target) in &mut config.targets {
            if target.name.is_empty() {
                target.name.clone_from(name);
            }
        }
        for (name, route) in &mut config.routes {
            if route.id.is_empty() {
                route.id.clone_from(name);
            }
        }
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, target) in &self.targets {
            if !self.clients.contains_key(&target.client) {
                anyhow::bail!(
                    "Target {} references unknown client {}",
                    name,
                    target.client
                );
            }
        }
        for (name, route) in &self.routes {
            for t in &route.targets {
                if !self.targets.contains_key(t) {
                    anyhow::bail!("Route {} references unknown target {}", name, t);
                }
            }
            let target_names = route.algorithm.target_names();
            for t in &target_names {
                if !self.targets.contains_key(t) {
                    anyhow::bail!("Route {} algorithm references unknown target {}", name, t);
                }
            }
        }
        Ok(())
    }
}

impl AlgorithmConfig {
    pub fn target_names(&self) -> Vec<String> {
        match self {
            AlgorithmConfig::Passthrough { target } => vec![target.clone()],
            AlgorithmConfig::Random { targets, .. } => targets.clone(),
            AlgorithmConfig::EpsilonGreedy { targets, .. } => targets.clone(),
            AlgorithmConfig::Thompson { targets } => targets.clone(),
            AlgorithmConfig::Ucb { targets, .. } => targets.clone(),
            AlgorithmConfig::Contextual { targets, .. } => targets.clone(),
            AlgorithmConfig::LlmClassifier {
                classifier_target,
                strong_target,
                weak_target,
                ..
            } => {
                let mut v = vec![
                    classifier_target.clone(),
                    strong_target.clone(),
                    weak_target.clone(),
                ];
                v.sort();
                v.dedup();
                v
            }
        }
    }
}
