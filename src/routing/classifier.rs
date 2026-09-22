use crate::config::TargetConfig;
use crate::protocol::Request;
use crate::routing::{fallback_targets, AlgorithmOutcome, RoutingAlgorithm};

/// Switchyard-style LLM-as-classifier routing.
/// A lightweight "classifier" model is asked to estimate prompt difficulty.
///
/// Selection uses sidecar-extracted prompt features when available: prompts
/// with `difficulty` at or above the threshold, or `is_sensitive` at or above
/// 0.7, go to the strong model; everything else goes to weak. When the
/// sidecar is unavailable (`request.features == None`), falls back to the
/// legacy heuristic: prompts longer than `200 * threshold` bytes go strong.
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
    fn needs_features(&self) -> bool {
        true
    }

    fn select(
        &self,
        request: &Request,
        targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome> {
        let go_strong = match &request.features {
            Some(f) => f.difficulty >= self.threshold || f.is_sensitive >= 0.7,
            // Legacy fallback (sidecar down): long prompts are hard prompts.
            None => request.llm_request.prompt_text().len() as f64 > 200.0 * self.threshold,
        };
        let selected = if go_strong { &self.strong } else { &self.weak };
        Ok(AlgorithmOutcome {
            selected: selected.name.clone(),
            fallbacks: fallback_targets(targets, &selected.name)
                .iter()
                .map(|t| t.name.clone())
                .collect(),
        })
    }
}
