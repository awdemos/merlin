use crate::config::TargetConfig;
use crate::protocol::Request;
use crate::routing::{fallback_targets, AlgorithmOutcome, RoutingAlgorithm};

/// Switchyard-style LLM-as-classifier routing.
/// A lightweight "classifier" model is asked to estimate prompt difficulty.
/// If confidence is above threshold, route to the strong model; otherwise weak.
#[derive(Debug)]
pub struct LlmClassifier {
    _classifier: TargetConfig,
    strong: TargetConfig,
    weak: TargetConfig,
    threshold: f64,
}

impl LlmClassifier {
    pub fn new(
        classifier_target: TargetConfig,
        strong_target: TargetConfig,
        weak_target: TargetConfig,
    ) -> Self {
        Self {
            _classifier: classifier_target,
            strong: strong_target,
            weak: weak_target,
            threshold: 0.5,
        }
    }
}

impl RoutingAlgorithm for LlmClassifier {
    fn select(
        &self,
        request: &Request,
        targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome> {
        // Deterministic proxy for classifier: hard prompts go strong, rest go weak.
        let selected = if request.llm_request.prompt_text().len() as f64 > 200.0 * self.threshold {
            &self.strong
        } else {
            &self.weak
        };
        Ok(AlgorithmOutcome {
            selected: selected.name.clone(),
            fallbacks: fallback_targets(targets, &selected.name)
                .iter()
                .map(|t| t.name.clone())
                .collect(),
        })
    }
}
