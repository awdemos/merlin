use crate::config::TargetConfig;
use crate::protocol::Request;
use crate::routing::{AlgorithmOutcome, RoutingAlgorithm};

#[derive(Debug)]
pub struct Passthrough {
    target: TargetConfig,
}

impl Passthrough {
    pub fn new(target: TargetConfig) -> Self {
        Self { target }
    }
}

impl RoutingAlgorithm for Passthrough {
    fn select(
        &self,
        _request: &Request,
        targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome> {
        Ok(AlgorithmOutcome {
            selected: self.target.name.clone(),
            fallbacks: crate::routing::fallback_targets(targets, &self.target.name)
                .iter()
                .map(|t| t.name.clone())
                .collect(),
        })
    }
}
