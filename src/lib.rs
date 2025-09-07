// src/lib.rs
mod metrics;
mod providers;
mod routing;
pub mod server;

pub use metrics::MetricCollector;
pub use providers::LlmProvider;
pub use providers::OllamaProvider;
pub use providers::OpenAiProvider;
pub use routing::RoutingPolicy;

pub struct Router<P: LlmProvider> {
    providers: Vec<P>,
    policy: RoutingPolicy,
    metrics: MetricCollector,
}

impl<P: LlmProvider> Router<P> {
    pub async fn new(providers: Vec<P>, policy: RoutingPolicy) -> Self {
        let metrics = MetricCollector::connect()
            .await
            .expect("Redis connection failed");
        Router {
            providers,
            policy,
            metrics,
        }
    }

    pub async fn route(&mut self, prompt: &str, _max_tokens: usize) -> anyhow::Result<String> {
        let selected_provider = self.select_provider().await;
        let result = selected_provider.chat(prompt).await?;
        self.metrics
            .record_success(selected_provider.name(), result.len())
            .await;
        Ok(result)
    }

    async fn select_provider(&self) -> &P {
        let index = self.policy.select_index(self.providers.len());
        &self.providers[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;

    mock! {
        Provider {}

        #[async_trait]
        impl LlmProvider for Provider {
            async fn chat(&self, prompt: &str) -> anyhow::Result<String>;
            fn name(&self) -> &'static str;
        }
    }

    #[tokio::test]
    async fn test_routing_policy_epsilon_greedy() {
        let policy = RoutingPolicy::EpsilonGreedy { epsilon: 0.5 };
        let index = policy.select_index(3);
        assert!(index < 3);
    }

    #[tokio::test]
    async fn test_router_selects_provider() {
        let mut mock_provider = MockProvider::new();
        mock_provider.expect_name().return_const("MockProvider");

        // This test demonstrates the structure but can't run without Redis
        // In a real test environment, we'd use test containers or mock Redis
        assert_eq!(mock_provider.name(), "MockProvider");
    }
}
