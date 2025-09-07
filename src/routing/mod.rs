use rand::Rng;

pub enum RoutingPolicy {
    EpsilonGreedy { epsilon: f64 },
    ThompsonSampling,
}

impl RoutingPolicy {
    pub fn select_index(&self, num_providers: usize) -> usize {
        match self {
            RoutingPolicy::EpsilonGreedy { epsilon } => {
                if rand::thread_rng().gen_bool(*epsilon) {
                    rand::thread_rng().gen_range(0..num_providers)
                } else {
                    0 // Default to first provider for now
                }
            }
            RoutingPolicy::ThompsonSampling => todo!(),
        }
    }
}
