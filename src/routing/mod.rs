//! Composable routing algorithms.

use crate::config::TargetConfig;
use crate::protocol::Request;

pub mod classifier;
pub mod contextual;
pub mod epsilon_greedy;
pub mod passthrough;
pub mod random;
pub mod thompson;
pub mod ucb;

pub use classifier::LlmClassifier;
pub use contextual::ContextualBandit;
pub use epsilon_greedy::EpsilonGreedy;
pub use passthrough::Passthrough;
pub use random::RandomRouter;
pub use thompson::ThompsonSampling;
pub use ucb::UpperConfidenceBound;

/// Outcome of a routing decision: selected target name + fallback names.
#[derive(Debug, Clone, Default)]
pub struct AlgorithmOutcome {
    pub selected: String,
    pub fallbacks: Vec<String>,
}

pub trait RoutingAlgorithm: Send + Sync + std::fmt::Debug {
    fn select(
        &self,
        request: &Request,
        targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome>;

    /// Record reward for a target selection. Default no-op.
    fn record_reward(&self, _target: &str, _request: &Request, _reward: f64) {}
}

/// Helpers to resolve fallback target configs by name.
pub fn fallback_targets(targets: &[TargetConfig], selected: &str) -> Vec<TargetConfig> {
    targets
        .iter()
        .filter(|t| t.name != selected)
        .cloned()
        .collect()
}

/// Choose a target by name from the list.
pub fn find_target<'a>(targets: &'a [TargetConfig], name: &str) -> Option<&'a TargetConfig> {
    targets.iter().find(|t| t.name == name)
}
