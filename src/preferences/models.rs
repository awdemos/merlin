use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub user_id: String,
    pub optimize_for: OptimizationTarget,
    pub max_tokens: u32,
    pub temperature: f32,
    pub custom_weights: HashMap<String, f32>,
    pub preferred_models: Vec<String>,
    pub excluded_models: Vec<String>,
    pub learning_enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub interaction_history: Vec<UserInteraction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationTarget {
    Quality,
    Speed,
    Cost,
    Balanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteraction {
    pub session_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub prompt_features: PromptInteractionFeatures,
    pub model_used: String,
    pub rating: Option<u8>,
    pub feedback_type: Option<String>,
    pub response_time_ms: Option<u32>,
    pub cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInteractionFeatures {
    pub domain_category: String,
    pub task_type: String,
    pub complexity_score: f64,
    pub estimated_tokens: u32,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceUpdateRequest {
    pub user_id: String,
    pub optimize_for: Option<OptimizationTarget>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub custom_weights: Option<HashMap<String, f32>>,
    pub preferred_models: Option<Vec<String>>,
    pub excluded_models: Option<Vec<String>>,
    pub learning_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceResponse {
    pub success: bool,
    pub preferences: Option<UserPreferences>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub user_id: String,
    pub total_requests: u32,
    pub average_rating: f32,
    pub preferred_models_usage: HashMap<String, u32>,
    pub cost_savings: f64,
    pub time_savings_ms: u32,
    pub learning_progress: f32,
}

impl Default for UserPreferences {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id: Uuid::new_v4().to_string(),
            optimize_for: OptimizationTarget::Balanced,
            max_tokens: 2048,
            temperature: 0.7,
            custom_weights: HashMap::new(),
            preferred_models: Vec::new(),
            excluded_models: Vec::new(),
            learning_enabled: true,
            created_at: now,
            updated_at: now,
            interaction_history: Vec::new(),
        }
    }
}

impl UserPreferences {
    pub fn new(user_id: String) -> Self {
        let mut prefs = Self::default();
        prefs.user_id = user_id;
        prefs
    }

    pub fn update_from_request(&mut self, request: PreferenceUpdateRequest) {
        if let Some(optimize_for) = request.optimize_for {
            self.optimize_for = optimize_for;
        }
        if let Some(max_tokens) = request.max_tokens {
            self.max_tokens = max_tokens;
        }
        if let Some(temperature) = request.temperature {
            self.temperature = temperature;
        }
        if let Some(custom_weights) = request.custom_weights {
            self.custom_weights = custom_weights;
        }
        if let Some(preferred_models) = request.preferred_models {
            self.preferred_models = preferred_models;
        }
        if let Some(excluded_models) = request.excluded_models {
            self.excluded_models = excluded_models;
        }
        if let Some(learning_enabled) = request.learning_enabled {
            self.learning_enabled = learning_enabled;
        }
        self.updated_at = chrono::Utc::now();
    }

    pub fn add_interaction(&mut self, interaction: UserInteraction) {
        self.interaction_history.push(interaction);
        // Keep only last 100 interactions to prevent unbounded growth
        if self.interaction_history.len() > 100 {
            self.interaction_history.remove(0);
        }
        self.updated_at = chrono::Utc::now();
    }

    pub fn calculate_model_preference_score(&self, model_name: &str, context_features: &[f64]) -> f64 {
        let mut score = 1.0; // Base score

        // Apply custom weights if available
        if let Some(&weight) = self.custom_weights.get(model_name) {
            score *= weight as f64;
        }

        // Preferred models get bonus
        if self.preferred_models.contains(&model_name.to_string()) {
            score *= 1.2;
        }

        // Excluded models get penalty
        if self.excluded_models.contains(&model_name.to_string()) {
            score *= 0.1;
        }

        // Apply optimization target
        match self.optimize_for {
            OptimizationTarget::Quality => {
                // Quality optimization prefers models with higher capability scores
                // This is a simplified version - in practice you'd use actual model capabilities
                score *= if model_name.contains("gpt-4") || model_name.contains("claude") { 1.3 } else { 1.0 };
            }
            OptimizationTarget::Speed => {
                // Speed optimization prefers faster models
                score *= if model_name.contains("llama") || model_name.contains("local") { 1.3 } else { 1.0 };
            }
            OptimizationTarget::Cost => {
                // Cost optimization prefers cheaper models
                score *= if model_name.contains("llama") || model_name.contains("local") { 1.4 } else { 1.0 };
            }
            OptimizationTarget::Balanced => {
                // Balanced approach considers all factors
                // Context features can help determine the best balance for this specific prompt
                if context_features.len() > 10 {
                    let complexity_factor = context_features[8]; // Assuming complexity is at index 8
                    if complexity_factor > 0.7 {
                        // High complexity tasks favor quality models
                        score *= if model_name.contains("gpt-4") || model_name.contains("claude") { 1.2 } else { 1.0 };
                    } else {
                        // Simple tasks can use cheaper models
                        score *= if model_name.contains("llama") { 1.2 } else { 1.0 };
                    }
                }
            }
        }

        score
    }

    pub fn calculate_personalization_strength(&self) -> f64 {
        let mut strength = 0.0;

        // Number of custom weights
        strength += (self.custom_weights.len() as f64 * 0.1).min(0.3);

        // Number of preferred/excluded models
        strength += ((self.preferred_models.len() + self.excluded_models.len()) as f64 * 0.1).min(0.3);

        // Interaction history provides learning data
        strength += (self.interaction_history.len() as f64 * 0.01).min(0.4);

        strength.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_preferences_creation() {
        let user_id = "test_user".to_string();
        let prefs = UserPreferences::new(user_id.clone());

        assert_eq!(prefs.user_id, user_id);
        assert_eq!(prefs.optimize_for, OptimizationTarget::Balanced);
        assert!(prefs.custom_weights.is_empty());
        assert!(prefs.interaction_history.is_empty());
    }

    #[test]
    fn test_preferences_update() {
        let mut prefs = UserPreferences::new("test_user".to_string());
        let original_updated_at = prefs.updated_at;

        let update = PreferenceUpdateRequest {
            user_id: "test_user".to_string(),
            optimize_for: Some(OptimizationTarget::Quality),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            custom_weights: Some({
                let mut weights = HashMap::new();
                weights.insert("gpt-4".to_string(), 1.5);
                weights
            }),
            preferred_models: Some(vec!["gpt-4".to_string()]),
            excluded_models: Some(vec!["llama-3.1".to_string()]),
            learning_enabled: Some(false),
        };

        prefs.update_from_request(update);

        assert_eq!(prefs.optimize_for, OptimizationTarget::Quality);
        assert_eq!(prefs.max_tokens, 4096);
        assert_eq!(prefs.temperature, 0.5);
        assert_eq!(prefs.custom_weights.get("gpt-4"), Some(&1.5));
        assert!(prefs.preferred_models.contains(&"gpt-4".to_string()));
        assert!(prefs.excluded_models.contains(&"llama-3.1".to_string()));
        assert!(!prefs.learning_enabled);
        assert!(prefs.updated_at > original_updated_at);
    }

    #[test]
    fn test_interaction_history() {
        let mut prefs = UserPreferences::new("test_user".to_string());

        let interaction = UserInteraction {
            session_id: "session_1".to_string(),
            timestamp: chrono::Utc::now(),
            prompt_features: PromptInteractionFeatures {
                domain_category: "Technical".to_string(),
                task_type: "Question".to_string(),
                complexity_score: 0.8,
                estimated_tokens: 150,
                keywords: vec!["code".to_string(), "algorithm".to_string()],
            },
            model_used: "gpt-4".to_string(),
            rating: Some(5),
            feedback_type: Some("Quality".to_string()),
            response_time_ms: Some(2500),
            cost: Some(0.03),
        };

        prefs.add_interaction(interaction.clone());
        assert_eq!(prefs.interaction_history.len(), 1);
        assert_eq!(prefs.interaction_history[0].rating, Some(5));

        // Test history limit
        for i in 0..105 {
            let mut interaction = interaction.clone();
            interaction.session_id = format!("session_{}", i);
            prefs.add_interaction(interaction);
        }

        assert_eq!(prefs.interaction_history.len(), 100);
    }

    #[test]
    fn test_model_preference_scoring() {
        let mut prefs = UserPreferences::new("test_user".to_string());

        // Add custom weights
        prefs.custom_weights.insert("gpt-4".to_string(), 1.5);
        prefs.preferred_models.push("claude-3".to_string());
        prefs.excluded_models.push("llama-3.1".to_string());

        let context_features = vec![0.0; 20]; // Dummy features

        let gpt4_score = prefs.calculate_model_preference_score("gpt-4", &context_features);
        let claude_score = prefs.calculate_model_preference_score("claude-3", &context_features);
        let llama_score = prefs.calculate_model_preference_score("llama-3.1", &context_features);

        assert!(gpt4_score > 1.4); // 1.5 custom weight
        assert!(claude_score > 1.1); // 1.2 preferred bonus
        assert!(llama_score < 0.2); // 0.1 excluded penalty
    }

    #[test]
    fn test_personalization_strength() {
        let mut prefs = UserPreferences::new("test_user".to_string());

        // Initially low personalization
        assert!(prefs.calculate_personalization_strength() < 0.1);

        // Add some customizations
        prefs.custom_weights.insert("gpt-4".to_string(), 1.5);
        prefs.preferred_models.push("claude-3".to_string());
        prefs.excluded_models.push("llama-3.1".to_string());

        // Should have moderate personalization now
        let strength = prefs.calculate_personalization_strength();
        assert!(strength > 0.3 && strength < 0.7);

        // Add interactions to increase strength
        for i in 0..50 {
            prefs.add_interaction(UserInteraction {
                session_id: format!("session_{}", i),
                timestamp: chrono::Utc::now(),
                prompt_features: PromptInteractionFeatures {
                    domain_category: "Technical".to_string(),
                    task_type: "Question".to_string(),
                    complexity_score: 0.5,
                    estimated_tokens: 100,
                    keywords: vec!["test".to_string()],
                },
                model_used: "gpt-4".to_string(),
                rating: Some(4),
                feedback_type: Some("Quality".to_string()),
                response_time_ms: Some(2000),
                cost: Some(0.02),
            });
        }

        // Should have high personalization now
        let final_strength = prefs.calculate_personalization_strength();
        assert!(final_strength > 0.7);
    }
}