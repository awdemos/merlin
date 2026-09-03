use crate::config::TargetConfig;
use crate::protocol::Request;
use crate::routing::{AlgorithmOutcome, RoutingAlgorithm};
use rand::distributions::{Distribution, WeightedIndex};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

#[derive(Debug)]
pub struct RandomRouter {
    targets: Vec<TargetConfig>,
    weights: Option<Vec<f64>>,
    rng: std::sync::Mutex<StdRng>,
}

impl RandomRouter {
    pub fn new(targets: Vec<TargetConfig>, seed: Option<u64>, weights: Option<Vec<f64>>) -> Self {
        let rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };
        Self {
            targets,
            weights,
            rng: std::sync::Mutex::new(rng),
        }
    }
}

impl RoutingAlgorithm for RandomRouter {
    fn select(
        &self,
        _request: &Request,
        _targets: &[TargetConfig],
    ) -> anyhow::Result<AlgorithmOutcome> {
        let mut rng = self.rng.lock().unwrap();
        let selected = if let Some(w) = &self.weights {
            let dist = WeightedIndex::new(w).map_err(|e| anyhow::anyhow!("bad weights: {}", e))?;
            self.targets[dist.sample(&mut *rng)].name.clone()
        } else {
            self.targets
                .choose(&mut *rng)
                .map(|t| t.name.clone())
                .unwrap_or_default()
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
}
