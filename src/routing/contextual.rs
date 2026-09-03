use crate::config::TargetConfig;
use crate::protocol::Request;
use crate::routing::{AlgorithmOutcome, RoutingAlgorithm};

#[derive(Debug)]
pub struct ContextualBandit {
    targets: Vec<TargetConfig>,
    #[allow(dead_code)]
    learning_rate: f64,
    #[allow(dead_code)]
    exploration_rate: f64,
}

impl ContextualBandit {
    pub fn new(
        targets: Vec<TargetConfig>,
        learning_rate: f64,
        exploration_rate: f64,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            targets,
            learning_rate,
            exploration_rate,
        })
    }
}

impl RoutingAlgorithm for ContextualBandit {
    fn select(
        &self,
        request: &Request,
        _targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome> {
        // Deterministic placeholder: hash prompt text to pick an index.
        let prompt = request.llm_request.prompt_text();
        let idx = if prompt.is_empty() {
            0
        } else {
            prompt.bytes().map(|b| b as usize).sum::<usize>() % self.targets.len().max(1)
        };
        let selected = self
            .targets
            .get(idx)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let fallbacks = self
            .targets
            .iter()
            .filter(|t| t.name != selected)
            .map(|t| t.name.clone())
            .collect();
        Ok(AlgorithmOutcome {
            selected,
            fallbacks,
        })
    }
}
