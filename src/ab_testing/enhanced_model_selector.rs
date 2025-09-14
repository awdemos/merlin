// src/ab_testing/enhanced_model_selector.rs
use crate::api::{
    ModelSelectRequest, ModelSelectResponse, ModelAlternative,
    DomainCategory,
};
use crate::preferences::models::UserPreferences;
use crate::features::{PromptFeatures, EmbeddingManager};
use crate::routing::RoutingPolicy;
use crate::metrics::MetricCollector;
use crate::ab_testing::{
    experiment::{ExperimentRunner, InteractionMetrics, ExperimentContext},
    config::ExperimentStatus,
};
use std::collections::HashMap;


pub struct EnhancedModelSelector {
    model_capabilities: HashMap<String, ModelCapabilities>,
    routing_policy: RoutingPolicy,
    metrics: MetricCollector,
    model_history: HashMap<String, ModelPerformanceHistory>,
    embedding_manager: EmbeddingManager,
    feature_dim: usize,
    experiment_runner: std::sync::Arc<tokio::sync::Mutex<ExperimentRunner>>,
}

#[derive(Debug, Clone)]
struct ModelCapabilities {
    pub name: String,
    pub strengths: Vec<DomainCategory>,
    pub cost_per_1k_tokens: f64,
    pub avg_latency_ms: u32,
    pub max_tokens: u32,
    pub quality_score: f32, // 0.0 - 1.0
    pub creativity_score: f32,
    pub reasoning_score: f32,
    pub code_score: f32,
}

#[derive(Debug, Default)]
struct ModelPerformanceHistory {
    pub total_requests: u32,
    pub successful_requests: u32,
    pub avg_rating: f32,
    pub avg_latency: f32,
    pub total_cost: f64,
}

impl EnhancedModelSelector {
    pub async fn new(experiment_runner: std::sync::Arc<tokio::sync::Mutex<ExperimentRunner>>) -> anyhow::Result<Self> {
        let metrics = MetricCollector::connect().await?;
        let embedding_manager = EmbeddingManager::new(256);

        // Initialize with common model capabilities
        let mut model_capabilities = HashMap::new();

        // OpenAI GPT-4
        model_capabilities.insert("gpt-4".to_string(), ModelCapabilities {
            name: "gpt-4".to_string(),
            strengths: vec![DomainCategory::General, DomainCategory::Technical, DomainCategory::Creative],
            cost_per_1k_tokens: 0.03,
            avg_latency_ms: 800,
            max_tokens: 8192,
            quality_score: 0.95,
            creativity_score: 0.90,
            reasoning_score: 0.92,
            code_score: 0.85,
        });

        // OpenAI GPT-3.5
        model_capabilities.insert("gpt-3.5-turbo".to_string(), ModelCapabilities {
            name: "gpt-3.5-turbo".to_string(),
            strengths: vec![DomainCategory::General, DomainCategory::Creative],
            cost_per_1k_tokens: 0.0015,
            avg_latency_ms: 300,
            max_tokens: 4096,
            quality_score: 0.85,
            creativity_score: 0.88,
            reasoning_score: 0.80,
            code_score: 0.75,
        });

        // Anthropic Claude
        model_capabilities.insert("claude-2".to_string(), ModelCapabilities {
            name: "claude-2".to_string(),
            strengths: vec![DomainCategory::General, DomainCategory::Technical, DomainCategory::Creative],
            cost_per_1k_tokens: 0.011,
            avg_latency_ms: 1200,
            max_tokens: 100000,
            quality_score: 0.92,
            creativity_score: 0.85,
            reasoning_score: 0.95,
            code_score: 0.88,
        });

        Ok(Self {
            model_capabilities,
            routing_policy: RoutingPolicy::new_contextual(3, 256, 0.01, 0.1),
            metrics,
            model_history: HashMap::new(),
            embedding_manager,
            feature_dim: 256,
            experiment_runner,
        })
    }

    pub async fn select_model_with_ab_testing(
        &mut self,
        request: &ModelSelectRequest,
        user_preferences: Option<&UserPreferences>,
        user_id: &str,
    ) -> anyhow::Result<(ModelSelectResponse, ExperimentContext)> {
        // First, check if user is participating in any A/B testing experiments
        let experiment_context = self.check_experiment_participation(user_id).await?;

        // Get available models based on request and user preferences
        let available_models = self.get_available_models(request, user_preferences)?;

        // If participating in experiment, use experiment routing
        let selected_model = if experiment_context.experiment_id != "no-experiment" {
            self.select_model_via_experiment(&available_models, &experiment_context).await?
        } else {
            // Use standard contextual bandit selection
            self.select_model_standard(&available_models, request, user_preferences).await?
        };

        let capabilities = self.model_capabilities.get(&selected_model)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", selected_model))?;

        // Generate features for contextual bandit learning
        let features = PromptFeatures::analyze(&request.messages);
        let prompt_text = self.extract_prompt_text(request);
        let prompt_embedding = self.embedding_manager.get_embedding(&prompt_text);

        // Combine feature vector with embedding for contextual bandit
        let mut context_features = features.to_vector();
        context_features.extend_from_slice(&prompt_embedding);

        // Calculate confidence
        let confidence = self.calculate_contextual_confidence(&context_features, &selected_model);

        // Generate reasoning
        let reasoning = self.generate_reasoning(capabilities, &features, &context_features);

        // Create alternatives
        let alternatives = self.create_alternatives(&available_models, &selected_model, &context_features);

        let response = ModelSelectResponse {
            recommended_model: selected_model.clone(),
            confidence,
            reasoning,
            alternatives,
            estimated_cost: None,
            estimated_latency_ms: None,
            session_id: request.session_id.clone(),
        };

        Ok((response, experiment_context))
    }

    async fn check_experiment_participation(&self, user_id: &str) -> anyhow::Result<ExperimentContext> {
        let runner = self.experiment_runner.lock().await;
        let mut active_experiment_ids = Vec::new();

        // Find all running experiments
        for (experiment_id, experiment) in runner.experiments.iter() {
            if experiment.config.status == ExperimentStatus::Running {
                active_experiment_ids.push(experiment_id.clone());
            }
        }

        // Drop the lock before calling get_variant_for_user
        drop(runner);

        // Check if user is eligible for any active experiment
        if !active_experiment_ids.is_empty() {
            let mut runner = self.experiment_runner.lock().await;
            for experiment_id in active_experiment_ids {
                if let Some(variant) = runner.get_variant_for_user(&experiment_id, user_id) {
                    return Ok(ExperimentContext {
                        experiment_id: experiment_id.clone(),
                        variant_id: variant.config.id.clone(),
                        routing_policy: variant.config.routing_policy.clone(),
                        user_assignment_time: chrono::Utc::now(),
                    });
                }
            }
        }

        // Return empty context if not participating
        Ok(ExperimentContext {
            experiment_id: "no-experiment".to_string(),
            variant_id: "control".to_string(),
            routing_policy: crate::ab_testing::config::RoutingPolicyConfig::Static { provider_index: 0 },
            user_assignment_time: chrono::Utc::now(),
        })
    }

    async fn select_model_via_experiment(
        &self,
        available_models: &[String],
        experiment_context: &ExperimentContext,
    ) -> anyhow::Result<String> {
        // If we're in an experiment, use the variant's routing policy
        if experiment_context.experiment_id != "no-experiment" {
            let runner = self.experiment_runner.lock().await;

            if let Some(experiment) = runner.experiments.get(&experiment_context.experiment_id) {
                if let Some(variant) = experiment.variants.get(&experiment_context.variant_id) {
                    // Use the variant's routing policy to select from available models
                    let model_index = variant.routing_policy.select_index(available_models.len());
                    return Ok(available_models[model_index].clone());
                }
            }
        }

        // Fallback to first model
        Ok(available_models[0].clone())
    }

    async fn select_model_standard(
        &mut self,
        available_models: &[String],
        request: &ModelSelectRequest,
        user_preferences: Option<&UserPreferences>,
    ) -> anyhow::Result<String> {
        // Generate features for contextual bandit
        let features = PromptFeatures::analyze(&request.messages);
        let prompt_text = self.extract_prompt_text(request);
        let prompt_embedding = self.embedding_manager.get_embedding(&prompt_text);

        // Combine feature vector with embedding for contextual bandit
        let mut context_features = features.to_vector();
        context_features.extend_from_slice(&prompt_embedding);

        // Use contextual bandit to select model
        let selected_index = self.routing_policy.select_index_with_context(
            available_models.len(),
            &context_features
        );

        Ok(available_models[selected_index].clone())
    }

    pub async fn record_interaction(
        &mut self,
        user_id: &str,
        model_name: &str,
        experiment_context: &ExperimentContext,
        response_time_ms: u32,
        success: bool,
        user_rating: Option<u8>,
        cost: f64,
        error_message: Option<String>,
    ) -> anyhow::Result<()> {
        // Record in standard metrics
        self.metrics.record_success(model_name, 0); // Placeholder for actual token count

        // Update routing policy with reward
        let reward = if success { 1.0 } else { 0.0 };
        let model_index = self.model_capabilities.keys().position(|name| name == model_name).unwrap_or(0);
        self.routing_policy.update_reward(model_index, success);

        // Record in A/B testing if participating
        if experiment_context.experiment_id != "no-experiment" {
            let interaction_metrics = InteractionMetrics {
                response_time_ms,
                success,
                user_rating,
                cost,
                error_message,
                timestamp: chrono::Utc::now(),
            };

            let mut runner = self.experiment_runner.lock().await;
            runner.record_interaction(&experiment_context.experiment_id, user_id, &interaction_metrics);
        }

        // Update model performance history
        if let Some(history) = self.model_history.get_mut(model_name) {
            history.total_requests += 1;
            if success {
                history.successful_requests += 1;
            }
            history.avg_latency = (history.avg_latency * (history.total_requests - 1) as f32 + response_time_ms as f32) / history.total_requests as f32;
            history.total_cost += cost;
            if let Some(rating) = user_rating {
                history.avg_rating = (history.avg_rating * (history.total_requests - 1) as f32 + rating as f32) / history.total_requests as f32;
            }
        } else {
            self.model_history.insert(model_name.to_string(), ModelPerformanceHistory {
                total_requests: 1,
                successful_requests: if success { 1 } else { 0 },
                avg_rating: user_rating.map(|r| r as f32).unwrap_or(0.0),
                avg_latency: response_time_ms as f32,
                total_cost: cost,
            });
        }

        Ok(())
    }

    fn get_available_models(
        &self,
        request: &ModelSelectRequest,
        user_preferences: Option<&UserPreferences>,
    ) -> anyhow::Result<Vec<String>> {
        let mut available_models = request.models.clone();

        // Filter by user preferences if available
        if let Some(preferences) = user_preferences {
            if !preferences.preferred_models.is_empty() {
                available_models.retain(|model| preferences.preferred_models.contains(model));
            }

            // Remove excluded models
            available_models.retain(|model| !preferences.excluded_models.contains(model));
        }

        // Ensure we have at least one model
        if available_models.is_empty() {
            available_models = vec!["gpt-3.5-turbo".to_string()]; // Fallback
        }

        Ok(available_models)
    }

    fn extract_prompt_text(&self, request: &ModelSelectRequest) -> String {
        request.messages
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn calculate_contextual_confidence(&self, context_features: &[f64], model_name: &str) -> f64 {
        // Simple confidence calculation based on model capabilities and context match
        if let Some(capabilities) = self.model_capabilities.get(model_name) {
            let base_confidence = capabilities.quality_score as f64;

            // Adjust based on feature alignment (simplified)
            let feature_alignment = if context_features.len() > 10 {
                context_features.iter().take(10).sum::<f64>() / 10.0
            } else {
                0.5
            };

            (base_confidence + feature_alignment) / 2.0
        } else {
            0.5 // Default confidence
        }
    }

    fn generate_reasoning(
        &self,
        capabilities: &ModelCapabilities,
        features: &PromptFeatures,
        _context_features: &[f64],
    ) -> String {
        let mut reasoning = format!(
            "Selected {} based on: ",
            capabilities.name
        );

        // Add capability-based reasoning
        if features.complexity_score > 0.7 {
            reasoning.push_str("high complexity task, ");
        }
        if features.estimated_tokens > 1000 {
            reasoning.push_str("long context required, ");
        }
        if capabilities.quality_score > 0.9 {
            reasoning.push_str("high quality model, ");
        }
        if capabilities.cost_per_1k_tokens < 0.01 {
            reasoning.push_str("cost-effective, ");
        }

        // Add contextual insight
        if _context_features.len() > 0 {
            let avg_feature = _context_features.iter().sum::<f64>() / _context_features.len() as f64;
            if avg_feature > 0.6 {
                reasoning.push_str("strong contextual match, ");
            }
        }

        // Remove trailing comma and space
        if reasoning.ends_with(", ") {
            reasoning.pop();
            reasoning.pop();
        }

        reasoning
    }

    fn create_alternatives(
        &self,
        available_models: &[String],
        selected_model: &str,
        context_features: &[f64],
    ) -> Vec<ModelAlternative> {
        available_models.iter()
            .filter(|model| model != &selected_model)
            .map(|model| {
                let capabilities = self.model_capabilities.get(model).unwrap();
                let confidence = self.calculate_contextual_confidence(context_features, model);

                ModelAlternative {
                    model: model.clone(),
                    confidence,
                    estimated_cost: None, // Would calculate based on actual token usage
                    estimated_latency_ms: Some(capabilities.avg_latency_ms),
                }
            })
            .collect()
    }
}