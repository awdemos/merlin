use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Top-level deployment configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MerlinConfig {
    #[serde(default)]
    pub server: ServerConfig,
    pub clients: HashMap<String, ClientConfig>,
    pub targets: HashMap<String, TargetConfig>,
    pub routes: HashMap<String, RouteConfig>,
    #[serde(default)]
    pub features: FeaturesConfig,
    #[serde(default)]
    pub governance: GovernanceConfig,
}

/// Feature-extraction sidecar (`laya`) integration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FeaturesConfig {
    pub enabled: bool,
    pub url: String,
    pub timeout_ms: u64,
    pub cache_entries: usize,
    pub cache_ttl_secs: u64,
    pub guard_enabled: bool,
    pub jailbreak_threshold: f64,
    pub prompt_injection_threshold: f64,
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "http://127.0.0.1:8765".to_string(),
            timeout_ms: 100,
            cache_entries: 512,
            cache_ttl_secs: 300,
            guard_enabled: false,
            jailbreak_threshold: 0.85,
            prompt_injection_threshold: 0.85,
        }
    }
}

/// Governance (policy gate, app tokens, quotas) configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct GovernanceConfig {
    pub enabled: bool,
    /// Redis URL for the app/token/usage store. Falls back to
    /// `MERLIN_REDIS_URL`, then `redis://127.0.0.1:6379`.
    pub redis_url: Option<String>,
    /// Sidecar `is_sensitive` probability at or above which a request
    /// counts as sensitive.
    pub sensitivity_threshold: f64,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            redis_url: None,
            sensitivity_threshold: 0.7,
        }
    }
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
    /// USD per 1M input tokens; backs daily-spend accounting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_input: Option<f64>,
    /// USD per 1M output tokens; backs daily-spend accounting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_output: Option<f64>,
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
        if !self.governance.sensitivity_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.governance.sensitivity_threshold)
        {
            anyhow::bail!(
                "governance.sensitivity_threshold must be finite and within 0.0..=1.0, got {}",
                self.governance.sensitivity_threshold
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governance_config_defaults_when_absent() {
        let config: MerlinConfig =
            toml::from_str("[clients]\n[targets]\n[routes]\n").expect("parse empty config");
        assert!(!config.governance.enabled);
        assert_eq!(config.governance.sensitivity_threshold, 0.7);
        assert!(config.governance.redis_url.is_none());
    }

    #[test]
    fn governance_config_parses_overrides() {
        let config: MerlinConfig = toml::from_str(
            "[clients]\n[targets]\n[routes]\n[governance]\nenabled = true\nsensitivity_threshold = 0.5\nredis_url = \"redis://10.0.0.1:6379\"\n",
        )
        .expect("parse governance config");
        assert!(config.governance.enabled);
        assert_eq!(config.governance.sensitivity_threshold, 0.5);
        assert_eq!(
            config.governance.redis_url.as_deref(),
            Some("redis://10.0.0.1:6379")
        );
    }

    #[test]
    fn governance_threshold_out_of_range_fails_validation() {
        let mut config: MerlinConfig = toml::from_str(
            "[clients]\n[targets]\n[routes]\n[governance]\nenabled = true\nsensitivity_threshold = 1.5\n",
        )
        .expect("parses");
        assert!(config.validate().is_err());
        config.governance.sensitivity_threshold = f64::NAN;
        assert!(config.validate().is_err());
        config.governance.sensitivity_threshold = 0.7;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn target_pricing_defaults_to_none() {
        let target: TargetConfig =
            toml::from_str("client = \"openai\"\nmodel = \"gpt-4\"\n").unwrap();
        assert!(target.price_input.is_none());
        assert!(target.price_output.is_none());
    }

    #[test]
    fn target_pricing_parses() {
        let target: TargetConfig = toml::from_str(
            "client = \"openai\"\nmodel = \"gpt-4\"\nprice_input = 30.0\nprice_output = 60.0\n",
        )
        .unwrap();
        assert_eq!(target.price_input, Some(30.0));
        assert_eq!(target.price_output, Some(60.0));
    }

    #[test]
    fn examples_governance_toml_parses() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/governance.toml");
        let config = MerlinConfig::load_from_file(path.to_str().unwrap())
            .expect("examples/governance.toml must parse");
        assert!(config.governance.enabled);
        assert!(config.routes.contains_key("default"));
        assert!(config.targets.contains_key("local_qwen"));
        assert!(config.validate().is_ok());
    }
}
