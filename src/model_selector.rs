// src/model_selector.rs
use crate::api::{
    ModelSelectRequest, ModelSelectResponse, ModelAlternative,
    DomainCategory, TaskType,
};
use crate::preferences::models::UserPreferences;
use crate::features::{PromptFeatures, EmbeddingManager};
use crate::routing::RoutingPolicy;
use crate::metrics::MetricCollector;
use std::collections::HashMap;
use uuid::Uuid;

pub struct IntelligentModelSelector {
    model_capabilities: HashMap<String, ModelCapabilities>,
    routing_policy: RoutingPolicy,
    metrics: MetricCollector,
    model_history: HashMap<String, ModelPerformanceHistory>,
    embedding_manager: EmbeddingManager,
    feature_dim: usize,
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

impl IntelligentModelSelector {
    pub async fn new() -> anyhow::Result<Self> {
        let metrics = MetricCollector::connect().await?;

        // Initialize with common model capabilities
        let mut model_capabilities = HashMap::new();

        // GPT-4 capabilities
        model_capabilities.insert(
            "gpt-4".to_string(),
            ModelCapabilities {
                name: "gpt-4".to_string(),
                strengths: vec![
                    DomainCategory::Analytical,
                    DomainCategory::Technical,
                    DomainCategory::Mathematical,
                ],
                cost_per_1k_tokens: 0.03,
                avg_latency_ms: 2500,
                max_tokens: 4096,
                quality_score: 0.95,
                creativity_score: 0.85,
                reasoning_score: 0.95,
                code_score: 0.90,
            }
        );

        // Claude-3 capabilities
        model_capabilities.insert(
            "claude-3".to_string(),
            ModelCapabilities {
                name: "claude-3".to_string(),
                strengths: vec![
                    DomainCategory::Creative,
                    DomainCategory::Analytical,
                    DomainCategory::General,
                ],
                cost_per_1k_tokens: 0.025,
                avg_latency_ms: 2000,
                max_tokens: 8192,
                quality_score: 0.92,
                creativity_score: 0.95,
                reasoning_score: 0.88,
                code_score: 0.85,
            }
        );

        // Llama-3.1 capabilities
        model_capabilities.insert(
            "llama-3.1".to_string(),
            ModelCapabilities {
                name: "llama-3.1".to_string(),
                strengths: vec![
                    DomainCategory::General,
                    DomainCategory::CodeGeneration,
                ],
                cost_per_1k_tokens: 0.01,
                avg_latency_ms: 1500,
                max_tokens: 2048,
                quality_score: 0.80,
                creativity_score: 0.75,
                reasoning_score: 0.78,
                code_score: 0.88,
            }
        );

        // Calculate feature dimension based on our feature vector
        let feature_dim = 4 + // Domain categories (one-hot)
                        4 + // Task types (one-hot)
                        2 + // Complexity and normalized tokens
                        24; // Keyword features

        // Initialize Contextual Bandit with available models
        let routing_policy = RoutingPolicy::new_contextual(
            model_capabilities.len(),
            feature_dim,
            0.1, // learning rate
            0.2, // exploration rate
        );

        // Initialize embedding manager
        let embedding_manager = EmbeddingManager::new(64); // 64-dimensional embeddings

        Ok(IntelligentModelSelector {
            model_capabilities,
            routing_policy,
            metrics,
            model_history: HashMap::new(),
            embedding_manager,
            feature_dim,
        })
    }
    
    pub async fn select_model(&mut self, request: ModelSelectRequest) -> anyhow::Result<ModelSelectResponse> {
        let session_id = request.session_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Analyze the prompt features using enhanced feature extraction
        let features = PromptFeatures::analyze(&request.messages);
        let feature_vector = features.to_feature_vector();

        // Generate embedding for the prompt
        let prompt_text = request.messages.iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let prompt_embedding = self.embedding_manager.get_embedding(&prompt_text);

        // Combine feature vector with embedding for contextual bandit
        let mut context_features = feature_vector;
        context_features.extend_from_slice(&prompt_embedding);

        // Use contextual bandit to select model
        let model_names: Vec<&String> = request.models.iter().collect();
        let provider_indices: Vec<usize> = (0..request.models.len()).collect();

        // Find the best model using contextual bandit
        let selected_index = self.routing_policy.select_index_with_context(
            model_names.len(),
            &context_features
        );

        let recommended_model = &request.models[selected_index];
        let capabilities = self.model_capabilities.get(recommended_model)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", recommended_model))?;

        // Calculate confidence based on contextual bandit prediction
        let confidence = self.calculate_contextual_confidence(&context_features, selected_index);

        // Generate reasoning using both traditional and contextual methods
        let reasoning = self.generate_contextual_reasoning(
            capabilities,
            &features,
            &context_features,
            selected_index,
        );

        // Create alternatives list with contextual scores
        let alternatives: Vec<ModelAlternative> = provider_indices.iter()
            .filter(|&&i| i != selected_index)
            .map(|&i| {
                let model_name = &request.models[i];
                let model_caps = self.model_capabilities.get(model_name).unwrap();
                let alt_confidence = self.calculate_contextual_confidence(&context_features, i);

                ModelAlternative {
                    model: model_name.clone(),
                    confidence: alt_confidence,
                    estimated_cost: Some(
                        model_caps.cost_per_1k_tokens * (features.estimated_tokens as f64 / 1000.0)
                    ),
                    estimated_latency_ms: Some(model_caps.avg_latency_ms),
                }
            })
            .collect();

        Ok(ModelSelectResponse {
            recommended_model: recommended_model.clone(),
            confidence,
            reasoning,
            alternatives,
            estimated_cost: Some(
                capabilities.cost_per_1k_tokens * (features.estimated_tokens as f64 / 1000.0)
            ),
            estimated_latency_ms: Some(capabilities.avg_latency_ms),
            session_id: Some(session_id),
        })
    }

    fn calculate_contextual_confidence(&self, context_features: &[f64], _provider_index: usize) -> f64 {
        // For contextual bandit, confidence is based on the prediction score
        // plus some heuristics about how well we understand this context

        // Base confidence from feature extraction quality
        let feature_confidence = if context_features.len() > 10 { 0.8 } else { 0.6 };

        // Adjust based on provider familiarity (more pulls = higher confidence)
        // This is a simplified version - in practice you'd track this per arm
        let familiarity_confidence = 0.7;

        (feature_confidence + familiarity_confidence) / 2.0
    }

    fn generate_contextual_reasoning(
        &self,
        capabilities: &ModelCapabilities,
        features: &PromptFeatures,
        context_features: &[f64],
        _selected_index: usize,
    ) -> String {
        let mut reasoning = Vec::new();

        // Traditional reasoning factors
        if capabilities.strengths.contains(&features.domain_category) {
            reasoning.push(format!(
                "Strong match for {:?} domain",
                features.domain_category
            ));
        }

        if features.complexity_score > 0.7 {
            reasoning.push("High complexity task requires advanced reasoning".to_string());
        }

        // Contextual reasoning
        let context_strength = context_features.iter().take(10) // Look at first 10 features
            .map(|&x| x.abs())
            .sum::<f64>() / 10.0;

        if context_strength > 0.5 {
            reasoning.push("Strong contextual features influence selection".to_string());
        }

        // Learning-based reasoning
        reasoning.push("Selection informed by contextual bandit learning".to_string());

        if reasoning.is_empty() {
            reasoning.push("Selected based on contextual analysis and historical performance".to_string());
        }

        reasoning.join("; ")
    }
    
    fn calculate_model_score(
        &self,
        capabilities: &ModelCapabilities,
        features: &PromptFeatures,
        preferences: Option<&UserPreferences>,
    ) -> f64 {
        let mut score = 0.0;
        
        // Base quality score
        score += capabilities.quality_score as f64 * 0.3;
        
        // Domain matching bonus
        if capabilities.strengths.contains(&features.domain_category) {
            score += 0.2;
        }
        
        // Task-specific scoring
        match features.task_type {
            TaskType::Analysis => score += capabilities.reasoning_score as f64 * 0.2,
            TaskType::Generation => score += capabilities.creativity_score as f64 * 0.2,
            TaskType::Question => score += capabilities.reasoning_score as f64 * 0.15,
            _ => score += 0.1,
        }
        
        // Complexity matching
        if features.complexity_score > 0.7 {
            // High complexity tasks favor more capable models
            score += capabilities.quality_score as f64 * 0.2;
        } else {
            // Simple tasks might not need the most expensive model
            score += (1.0 - capabilities.cost_per_1k_tokens / 0.05) * 0.1;
        }
        
        // Apply user preferences
        if let Some(prefs) = preferences {
            match prefs.optimize_for {
                crate::preferences::models::OptimizationTarget::Quality => {
                    score += capabilities.quality_score as f64 * 0.3;
                }
                crate::preferences::models::OptimizationTarget::Speed => {
                    // Favor models with lower latency
                    let speed_score = 1.0 - (capabilities.avg_latency_ms as f64 / 5000.0).min(1.0);
                    score += speed_score * 0.3;
                }
                crate::preferences::models::OptimizationTarget::Cost => {
                    // Favor cheaper models
                    let cost_score = 1.0 - (capabilities.cost_per_1k_tokens / 0.05).min(1.0);
                    score += cost_score * 0.3;
                }
                crate::preferences::models::OptimizationTarget::Balanced => {
                    // Balanced approach
                    let speed_score = 1.0 - (capabilities.avg_latency_ms as f64 / 5000.0).min(1.0);
                    let cost_score = 1.0 - (capabilities.cost_per_1k_tokens / 0.05).min(1.0);
                    score += (speed_score + cost_score) * 0.1;
                }
            }
            
            // Apply custom weights if provided
            let weights = &prefs.custom_weights;
            if let Some(weight) = weights.get(&capabilities.name) {
                score *= *weight as f64;
            }
        }
        
        // Historical performance adjustment
        if let Some(history) = self.model_history.get(&capabilities.name) {
            if history.total_requests > 0 {
                let success_rate = history.successful_requests as f64 / history.total_requests as f64;
                let rating_bonus = ((history.avg_rating - 3.0) / 2.0) as f64; // Scale 1-5 to -1 to 1
                score += (success_rate * 0.1) + (rating_bonus * 0.1);
            }
        }
        
        score.max(0.0).min(1.0)
    }
    
    fn generate_reasoning(
        &self,
        capabilities: &ModelCapabilities,
        features: &PromptFeatures,
        score: f64,
    ) -> String {
        let mut reasoning = Vec::new();
        
        if capabilities.strengths.contains(&features.domain_category) {
            reasoning.push(format!(
                "Strong match for {:?} domain",
                features.domain_category
            ));
        }
        
        if features.complexity_score > 0.7 {
            reasoning.push("High complexity task requires advanced reasoning".to_string());
        }
        
        if features.estimated_tokens > 2000 {
            reasoning.push("Long response expected, suitable model context".to_string());
        }
        
        match features.task_type {
            TaskType::Analysis => reasoning.push("Analytical task benefits from strong reasoning".to_string()),
            TaskType::Generation => reasoning.push("Creative generation task".to_string()),
            TaskType::Question => reasoning.push("Question answering task".to_string()),
            _ => {}
        }
        
        if reasoning.is_empty() {
            reasoning.push(format!(
                "Selected based on overall capability score ({:.2})",
                score
            ));
        }
        
        reasoning.join("; ")
    }
}
