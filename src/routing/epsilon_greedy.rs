use crate::config::TargetConfig;
use crate::protocol::Request;
use crate::routing::{AlgorithmOutcome, RoutingAlgorithm};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct EpsilonGreedy {
    targets: Vec<TargetConfig>,
    epsilon: f64,
    counts: Mutex<HashMap<String, u64>>,
    rewards: Mutex<HashMap<String, f64>>,
}

impl EpsilonGreedy {
    pub fn new(targets: Vec<TargetConfig>, epsilon: f64) -> Self {
        Self {
            targets,
            epsilon,
            counts: Mutex::new(HashMap::new()),
            rewards: Mutex::new(HashMap::new()),
        }
    }
}

impl RoutingAlgorithm for EpsilonGreedy {
    fn select(
        &self,
        _request: &Request,
        _targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome> {
        let counts = self.counts.lock().unwrap();
        let rewards = self.rewards.lock().unwrap();
        let mut best: Option<(&str, f64)> = None;
        for t in &self.targets {
            let n = *counts.get(&t.name).unwrap_or(&0) as f64;
            let r = *rewards.get(&t.name).unwrap_or(&0.0);
            let score = if n > 0.0 { r / n } else { 0.0 };
            let is_better = if let Some((_, s)) = best {
                score > s
            } else {
                true
            };
            if is_better {
                best = Some((&t.name, score));
            }
        }
        let selected = best.map(|(n, _)| n.to_string()).unwrap_or_else(|| {
            self.targets
                .first()
                .map(|t| t.name.clone())
                .unwrap_or_default()
        });
        drop(counts);
        drop(rewards);

        let explore: f64 = rand::random();
        let selected = if explore < self.epsilon {
            let idx = rand::random::<usize>() % self.targets.len().max(1);
            self.targets
                .get(idx)
                .map(|t| t.name.clone())
                .unwrap_or(selected)
        } else {
            selected
        };

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
        *self
            .counts
            .lock()
            .unwrap()
            .entry(target.to_string())
            .or_insert(0) += 1;
        *self
            .rewards
            .lock()
            .unwrap()
            .entry(target.to_string())
            .or_insert(0.0) += reward;
    }
}
