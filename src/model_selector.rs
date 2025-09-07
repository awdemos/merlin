// src/model_selector.rs
use crate::api::{
    ModelSelectRequest, ModelSelectResponse, ModelAlternative, PromptFeatures,
    OptimizationTarget, DomainCategory, TaskType, UserPreferences,
};
use crate::routing::RoutingPolicy;
use crate::metrics::MetricCollector;
use std::collections::HashMap;
use uuid::Uuid;

pub struct IntelligentModelSelector {
    model_capabilities: HashMap<String, ModelCapabilities>,
    routing_policy: RoutingPolicy,
    metrics: MetricCollector,
    model_history: HashMap<String, ModelPerformanceHistory>,
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
        
        // Initialize Thompson Sampling with available models
        let routing_policy = RoutingPolicy::new_thompson_sampling(model_capabilities.len());
        
        Ok(IntelligentModelSelector {
            model_capabilities,
            routing_policy,
            metrics,
            model_history: HashMap::new(),
        })
    }
    
    pub async fn select_model(&self, request: ModelSelectRequest) -> anyhow::Result<ModelSelectResponse> {
        let session_id = request.session_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // Analyze the prompt features
        let features = PromptFeatures::analyze(&request.messages);
        
        // Score each requested model
        let mut model_scores: Vec<(String, f64, String)> = Vec::new();
        
        for model_name in &request.models {
            if let Some(capabilities) = self.model_capabilities.get(model_name) {
                let score = self.calculate_model_score(
                    capabilities,
                    &features,
                    request.preferences.as_ref(),
                );
                let reasoning = self.generate_reasoning(capabilities, &features, score);
                model_scores.push((model_name.clone(), score, reasoning));
            }
        }
        
        // Sort by score (highest first)
        model_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        if model_scores.is_empty() {
            return Err(anyhow::anyhow!("No valid models found"));
        }
        
        let (recommended_model, confidence, reasoning) = model_scores[0].clone();
        
        // Create alternatives list
        let alternatives: Vec<ModelAlternative> = model_scores
            .iter()
            .skip(1)
            .map(|(model, score, _)| {
                let capabilities = self.model_capabilities.get(model).unwrap();
                ModelAlternative {
                    model: model.clone(),
                    confidence: *score,
                    estimated_cost: Some(
                        capabilities.cost_per_1k_tokens * (features.estimated_tokens as f64 / 1000.0)
                    ),
                    estimated_latency_ms: Some(capabilities.avg_latency_ms),
                }
            })
            .collect();
        
        let recommended_capabilities = self.model_capabilities.get(&recommended_model).unwrap();
        
        Ok(ModelSelectResponse {
            recommended_model: recommended_model.clone(),
            confidence,
            reasoning,
            alternatives,
            estimated_cost: Some(
                recommended_capabilities.cost_per_1k_tokens * (features.estimated_tokens as f64 / 1000.0)
            ),
            estimated_latency_ms: Some(recommended_capabilities.avg_latency_ms),
            session_id: Some(session_id),
        })
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
            match prefs.optimize_for.as_ref().unwrap_or(&OptimizationTarget::Balanced) {
                OptimizationTarget::Quality => {
                    score += capabilities.quality_score as f64 * 0.3;
                }
                OptimizationTarget::Speed => {
                    // Favor models with lower latency
                    let speed_score = 1.0 - (capabilities.avg_latency_ms as f64 / 5000.0).min(1.0);
                    score += speed_score * 0.3;
                }
                OptimizationTarget::Cost => {
                    // Favor cheaper models
                    let cost_score = 1.0 - (capabilities.cost_per_1k_tokens / 0.05).min(1.0);
                    score += cost_score * 0.3;
                }
                OptimizationTarget::Balanced => {
                    // Balanced approach
                    let speed_score = 1.0 - (capabilities.avg_latency_ms as f64 / 5000.0).min(1.0);
                    let cost_score = 1.0 - (capabilities.cost_per_1k_tokens / 0.05).min(1.0);
                    score += (speed_score + cost_score) * 0.1;
                }
            }
            
            // Apply custom weights if provided
            if let Some(weights) = &prefs.custom_weights {
                if let Some(weight) = weights.get(&capabilities.name) {
                    score *= *weight as f64;
                }
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
