use crate::config::TargetConfig;
use crate::protocol::Request;
use crate::routing::{AlgorithmOutcome, RoutingAlgorithm};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct ThompsonSampling {
    targets: Vec<TargetConfig>,
    successes: Mutex<HashMap<String, u64>>,
    failures: Mutex<HashMap<String, u64>>,
}

impl ThompsonSampling {
    pub fn new(targets: Vec<TargetConfig>) -> Self {
        Self {
            targets,
            successes: Mutex::new(HashMap::new()),
            failures: Mutex::new(HashMap::new()),
        }
    }
}

impl RoutingAlgorithm for ThompsonSampling {
    fn select(
        &self,
        _request: &Request,
        _targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome> {
        let successes = self.successes.lock().unwrap();
        let failures = self.failures.lock().unwrap();
        let mut best: Option<(String, f64)> = None;
        for t in &self.targets {
            let alpha = *successes.get(&t.name).unwrap_or(&0) as f64 + 1.0;
            let beta = *failures.get(&t.name).unwrap_or(&0) as f64 + 1.0;
            let sample =
                rand::random::<f64>().powf(1.0 / alpha) * rand::random::<f64>().powf(1.0 / beta);
            let is_better = if let Some((_, s)) = best {
                sample > s
            } else {
                true
            };
            if is_better {
                best = Some((t.name.clone(), sample));
            }
        }
        let selected = best.map(|(n, _)| n).unwrap_or_else(|| {
            self.targets
                .first()
                .map(|t| t.name.clone())
                .unwrap_or_default()
        });
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

    fn record_reward(&self, target: &str, _request: &Request, reward: f64) {
        if reward > 0.0 {
            *self
                .successes
                .lock()
                .unwrap()
                .entry(target.to_string())
                .or_insert(0) += 1;
        } else {
            *self
                .failures
                .lock()
                .unwrap()
                .entry(target.to_string())
                .or_insert(0) += 1;
        }
    }
}
