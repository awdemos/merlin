//! # Merlin - Intelligent Multi-Provider LLM Router
//!
//! Merlin is an AI routing system that intelligently selects optimal language models
//! based on real-time performance metrics. It supports multiple LLM providers and
//! uses various routing policies (Epsilon-Greedy, Thompson Sampling, UCB) to
//! distribute requests efficiently.
//!
//! ## Features
//!
//! - **Multi-Provider Support**: OpenAI, Ollama, and extensible provider system
//! - **Smart Routing**: Epsilon-Greedy, Thompson Sampling, and UCB algorithms
//! - **Real-Time Metrics**: Latency, cost, and quality tracking via Redis
//! - **A/B Testing**: Built-in experiment framework for model comparison
//!
//! ## Example
//!
//! ```rust,ignore
//! use merlin::{Router, RoutingPolicy, OpenAiProvider, LlmProvider};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let providers = vec![OpenAiProvider::new("sk-...".into(), "gpt-4".into())];
//!     let policy = RoutingPolicy::new_thompson_sampling(providers.len());
//!     let mut router = Router::new(providers, policy).await;
//!     
//!     let response = router.route("Hello, world!", 100).await?;
//!     println!("Response: {}", response);
//!     Ok(())
//! }
//! ```

pub mod ab_testing;
pub mod api;
mod feedback;
pub mod feature_numbering;
pub mod features;
pub mod integration;
mod metrics;
pub mod models;
mod model_selector;
pub mod performance;
pub mod preferences;
mod providers;
pub mod routing;
pub mod security;
pub mod server;
pub mod services;


pub use feedback::FeedbackProcessor;
pub use metrics::MetricCollector;
pub use model_selector::IntelligentModelSelector;
pub use providers::LlmProvider;
pub use providers::OllamaProvider;
pub use providers::OpenAiProvider;
pub use routing::RoutingPolicy;

/// Intelligent router that selects the best LLM provider based on performance metrics.
///
/// The router uses a configurable routing policy to distribute requests across
/// multiple LLM providers, tracking success/failure metrics to optimize selection.
///
/// # Type Parameters
///
/// * `P` - LLM provider implementation that implements [`LlmProvider`]
pub struct Router<P: LlmProvider> {
    providers: Vec<P>,
    policy: RoutingPolicy,
    metrics: MetricCollector,
}

impl<P: LlmProvider> Router<P> {
    /// Creates a new router with the given providers and routing policy.
    ///
    /// Connects to Redis for metrics storage. Panics if Redis connection fails.
    ///
    /// # Arguments
    ///
    /// * `providers` - Vector of LLM provider instances
    /// * `policy` - Routing policy for provider selection
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

    /// Routes a prompt to the best available provider and returns the response.
    ///
    /// Uses the routing policy to select a provider, sends the prompt,
    /// records metrics, and updates the policy based on success/failure.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The input text to send to the LLM
    /// * `_max_tokens` - Maximum tokens for the response (currently unused)
    ///
    /// # Errors
    ///
    /// Returns an error if the selected provider fails to generate a response.
    pub async fn route(&mut self, prompt: &str, _max_tokens: usize) -> anyhow::Result<String> {
        let provider_index = self.policy.select_index(self.providers.len());
        let selected_provider = &self.providers[provider_index];
        
        let result = selected_provider.chat(prompt).await;
        
        match result {
            Ok(response) => {
                // Record success metrics
                self.metrics
                    .record_success(selected_provider.name(), response.len())
                    .await;
                
                // Update routing policy with positive reward
                self.policy.update_reward(provider_index, true);
                
                Ok(response)
            }
            Err(e) => {
                // Record failure and update policy with negative reward
                self.metrics
                    .record_failure(selected_provider.name())
                    .await;
                
                self.policy.update_reward(provider_index, false);
                
                Err(e)
            }
        }
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
            fn model(&self) -> &str;
            fn capabilities(&self) -> Option<&'static crate::providers::ModelCapabilities>;
        }
    }

    #[tokio::test]
    async fn test_routing_policy_epsilon_greedy() {
        let policy = RoutingPolicy::EpsilonGreedy { epsilon: 0.5 };
        let index = policy.select_index(3);
        assert!(index < 3);
    }

    #[tokio::test]
    async fn test_thompson_sampling_policy() {
        let policy = RoutingPolicy::new_thompson_sampling(3);
        let index = policy.select_index(3);
        assert!(index < 3);
    }

    #[tokio::test]
    async fn test_thompson_arm_learning() {
        let mut arm = super::routing::ThompsonArm::new();
        
        // Initial state
        assert_eq!(arm.alpha, 1.0);
        assert_eq!(arm.beta, 1.0);
        
        // Update with success
        arm.update_success();
        assert_eq!(arm.alpha, 2.0);
        assert_eq!(arm.beta, 1.0);
        
        // Update with failure
        arm.update_failure();
        assert_eq!(arm.alpha, 2.0);
        assert_eq!(arm.beta, 2.0);
    }

    #[tokio::test]
    async fn test_policy_reward_updates() {
        let mut policy = RoutingPolicy::new_thompson_sampling(2);
        
        // Update with success for provider 0
        policy.update_reward(0, true);
        
        if let RoutingPolicy::ThompsonSampling { arms } = &policy {
            let arm = arms.get(&0).unwrap();
            assert_eq!(arm.alpha, 2.0);
            assert_eq!(arm.beta, 1.0);
        } else {
            panic!("Expected Thompson Sampling policy");
        }
        
        // Update with failure for provider 1
        policy.update_reward(1, false);
        
        if let RoutingPolicy::ThompsonSampling { arms } = &policy {
            let arm = arms.get(&1).unwrap();
            assert_eq!(arm.alpha, 1.0);
            assert_eq!(arm.beta, 2.0);
        }
    }

    #[tokio::test]
    async fn test_upper_confidence_bound_policy() {
        let policy = RoutingPolicy::new_upper_confidence_bound(3, 2.0);
        let index = policy.select_index(3);
        assert!(index < 3);
        
        if let RoutingPolicy::UpperConfidenceBound { arms, confidence_level, total_rounds } = &policy {
            assert_eq!(*confidence_level, 2.0);
            assert_eq!(*total_rounds, 0);
            assert_eq!(arms.len(), 3);
        } else {
            panic!("Expected UCB policy");
        }
    }

    #[tokio::test]
    async fn test_ucb_reward_updates() {
        let mut policy = RoutingPolicy::new_upper_confidence_bound(2, 1.5);
        
        // Update with success for provider 0
        policy.update_reward(0, true);
        
        if let RoutingPolicy::UpperConfidenceBound { arms, total_rounds, .. } = &policy {
            let arm = arms.get(&0).unwrap();
            assert_eq!(arm.num_pulls, 1);
            assert_eq!(arm.total_reward, 1.0);
            assert_eq!(arm.average_reward, 1.0);
            assert_eq!(*total_rounds, 1);
        } else {
            panic!("Expected UCB policy");
        }
        
        // Update with failure for provider 1
        policy.update_reward(1, false);
        
        if let RoutingPolicy::UpperConfidenceBound { arms, total_rounds, .. } = &policy {
            let arm = arms.get(&1).unwrap();
            assert_eq!(arm.num_pulls, 1);
            assert_eq!(arm.total_reward, 0.0);
            assert_eq!(arm.average_reward, 0.0);
            assert_eq!(*total_rounds, 2);
        }
    }

    #[tokio::test]
    async fn test_ucb_with_scores() {
        let mut policy = RoutingPolicy::new_upper_confidence_bound(2, 1.0);
        
        // Update with high score (0.8)
        policy.update_reward_with_score(0, 0.8);
        
        if let RoutingPolicy::UpperConfidenceBound { arms, .. } = &policy {
            let arm = arms.get(&0).unwrap();
            assert_eq!(arm.average_reward, 0.8);
        }
        
        // Update with low score (0.3)
        policy.update_reward_with_score(0, 0.3);
        
        if let RoutingPolicy::UpperConfidenceBound { arms, .. } = &policy {
            let arm = arms.get(&0).unwrap();
            assert_eq!(arm.average_reward, 0.55); // (0.8 + 0.3) / 2
        }
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
