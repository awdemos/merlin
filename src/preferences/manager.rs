use crate::feedback::FeedbackProcessor;
use crate::features::EmbeddingManager;
use crate::preferences::models::*;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PreferenceManager {
    redis_client: Option<Arc<redis::Client>>,
    preferences_cache: Arc<Mutex<HashMap<String, UserPreferences>>>,
    embedding_manager: EmbeddingManager,
    feedback_processor: Arc<Mutex<FeedbackProcessor>>,
}

impl PreferenceManager {
    pub async fn new() -> anyhow::Result<Self> {
        let redis_client = match redis::Client::open("redis://127.0.0.1:6379") {
            Ok(client) => Some(Arc::new(client)),
            Err(e) => {
                eprintln!("Warning: Could not connect to Redis for preferences: {}", e);
                None
            }
        };

        let feedback_processor = match FeedbackProcessor::new().await {
            Ok(processor) => Arc::new(Mutex::new(processor)),
            Err(e) => {
                eprintln!("Warning: Could not initialize feedback processor: {}", e);
                Arc::new(Mutex::new(FeedbackProcessor::new_fallback()))
            }
        };

        let embedding_manager = EmbeddingManager::new(64);

        Ok(Self {
            redis_client,
            preferences_cache: Arc::new(Mutex::new(HashMap::new())),
            embedding_manager,
            feedback_processor,
        })
    }

    pub async fn get_preferences(&self, user_id: &str) -> anyhow::Result<UserPreferences> {
        // Check cache first
        {
            let cache = self.preferences_cache.lock().await;
            if let Some(prefs) = cache.get(user_id) {
                return Ok(prefs.clone());
            }
        }

        // Try Redis if available
        if let Some(ref client) = self.redis_client {
            let mut conn = client.get_async_connection().await?;
            let key = format!("user_preferences:{}", user_id);

            if let Ok(prefs_json) = conn.get::<_, String>(key).await {
                let preferences: UserPreferences = serde_json::from_str(&prefs_json)?;

                // Update cache
                {
                    let mut cache = self.preferences_cache.lock().await;
                    cache.insert(user_id.to_string(), preferences.clone());
                }

                return Ok(preferences);
            }
        }

        // Return default preferences
        let default_prefs = UserPreferences::new(user_id.to_string());

        // Cache the default
        {
            let mut cache = self.preferences_cache.lock().await;
            cache.insert(user_id.to_string(), default_prefs.clone());
        }

        Ok(default_prefs)
    }

    pub async fn update_preferences(&self, request: PreferenceUpdateRequest) -> anyhow::Result<PreferenceResponse> {
        let user_id = &request.user_id;

        // Get existing preferences or create new ones
        let mut preferences = self.get_preferences(user_id).await?;

        // Update preferences
        preferences.update_from_request(request.clone());

        // Save to Redis if available
        if let Some(ref client) = self.redis_client {
            let mut conn = client.get_async_connection().await?;
            let key = format!("user_preferences:{}", user_id);
            let prefs_json = serde_json::to_string(&preferences)?;

            conn.set::<_, _, ()>(key, prefs_json).await?;

            // Set expiration (30 days)
            let expire_key = format!("user_preferences:{}", user_id);
            conn.expire::<_, ()>(expire_key, 60 * 60 * 24 * 30).await?;
        }

        // Update cache
        {
            let mut cache = self.preferences_cache.lock().await;
            cache.insert(user_id.to_string(), preferences.clone());
        }

        Ok(PreferenceResponse {
            success: true,
            preferences: Some(preferences),
            message: "Preferences updated successfully".to_string(),
        })
    }

    pub async fn record_user_interaction(&self, user_id: &str, interaction: UserInteraction) -> anyhow::Result<()> {
        // Get user preferences
        let mut preferences = self.get_preferences(user_id).await?;

        // Add interaction to history
        preferences.add_interaction(interaction.clone());

        // Update preferences with learned insights if learning is enabled
        if preferences.learning_enabled {
            self.learn_from_interaction(&mut preferences, &interaction).await?;
        }

        // Save updated preferences
        let update_request = PreferenceUpdateRequest {
            user_id: user_id.to_string(),
            optimize_for: None,
            max_tokens: None,
            temperature: None,
            custom_weights: None,
            preferred_models: None,
            excluded_models: None,
            learning_enabled: None,
        };

        self.update_preferences(update_request).await?;

        // Also record in feedback processor for broader learning
        if let Some(rating) = interaction.rating {
            if let Ok(mut processor) = self.feedback_processor.try_lock() {
                // Create feedback request from interaction
                let feedback_request = crate::api::FeedbackRequest {
                    session_id: interaction.session_id,
                    model_used: interaction.model_used,
                    rating,
                    feedback_type: crate::api::FeedbackType::Overall,
                    comment: None,
                    metadata: None,
                };

                // Process feedback (ignore errors for now)
                let _ = processor.process_feedback(&feedback_request).await;
            }
        }

        Ok(())
    }

    async fn learn_from_interaction(&self, preferences: &mut UserPreferences, interaction: &UserInteraction) -> anyhow::Result<()> {
        // Learn model preferences based on ratings and response metrics
        if let Some(rating) = interaction.rating {
            let model_name = &interaction.model_used;

            // Adjust custom weights based on rating
            let current_weight = preferences.custom_weights.get(model_name).copied().unwrap_or(1.0);

            // Learning rate based on rating (higher ratings = stronger learning)
            let learning_rate = match rating {
                5 => 0.2,  // Excellent - strong positive learning
                4 => 0.1,  // Good - moderate positive learning
                3 => 0.05, // Average - minimal learning
                2 => -0.1, // Poor - negative learning
                1 => -0.2, // Terrible - strong negative learning
                _ => 0.0,  // Unknown - no learning
            };

            let new_weight = (current_weight + learning_rate).max(0.1).min(3.0);
            preferences.custom_weights.insert(model_name.clone(), new_weight);

            // Learn optimization target preferences
            if rating >= 4 {
                // High rating - reinforce current optimization target
                // Or learn what optimization target would have been best for this interaction
                self.learn_optimization_target(preferences, interaction).await?;
            } else if rating <= 2 {
                // Low rating - try different optimization target
                self.adjust_optimization_target(preferences, interaction).await?;
            }
        }

        // Learn from response time
        if let Some(response_time) = interaction.response_time_ms {
            if response_time > 3000 && preferences.optimize_for == OptimizationTarget::Speed {
                // Too slow for speed optimization - suggest cost or balanced
                preferences.optimize_for = OptimizationTarget::Balanced;
            }
        }

        // Learn from cost
        if let Some(cost) = interaction.cost {
            if cost > 0.05 && preferences.optimize_for == OptimizationTarget::Cost {
                // Too expensive for cost optimization - suggest balanced or speed
                preferences.optimize_for = OptimizationTarget::Balanced;
            }
        }

        Ok(())
    }

    async fn learn_optimization_target(&self, preferences: &mut UserPreferences, interaction: &UserInteraction) -> anyhow::Result<()> {
        // Analyze the interaction to determine the best optimization target
        let prompt_features = &interaction.prompt_features;

        // High complexity, high cost tasks -> Quality optimization
        if prompt_features.complexity_score > 0.7 &&
           interaction.cost.unwrap_or(0.0) > 0.02 &&
           interaction.rating.unwrap_or(0) >= 4 {
            preferences.optimize_for = OptimizationTarget::Quality;
        }

        // Simple, fast tasks with good ratings -> Speed optimization
        else if prompt_features.complexity_score < 0.4 &&
                interaction.response_time_ms.unwrap_or(0) < 1500 &&
                interaction.rating.unwrap_or(0) >= 4 {
            preferences.optimize_for = OptimizationTarget::Speed;
        }

        // Low cost tasks with good ratings -> Cost optimization
        else if interaction.cost.unwrap_or(f64::INFINITY) < 0.01 &&
                interaction.rating.unwrap_or(0) >= 4 {
            preferences.optimize_for = OptimizationTarget::Cost;
        }

        Ok(())
    }

    async fn adjust_optimization_target(&self, preferences: &mut UserPreferences, interaction: &UserInteraction) -> anyhow::Result<()> {
        // If current optimization target is not working well, try a different one
        let current_target = preferences.optimize_for.clone();

        let new_target = match current_target {
            OptimizationTarget::Quality => OptimizationTarget::Balanced,
            OptimizationTarget::Speed => OptimizationTarget::Balanced,
            OptimizationTarget::Cost => OptimizationTarget::Balanced,
            OptimizationTarget::Balanced => {
                // Try to determine what would be better based on the interaction
                if interaction.response_time_ms.unwrap_or(0) > 3000 {
                    OptimizationTarget::Speed
                } else if interaction.cost.unwrap_or(f64::INFINITY) > 0.03 {
                    OptimizationTarget::Cost
                } else {
                    OptimizationTarget::Quality
                }
            }
        };

        preferences.optimize_for = new_target;
        Ok(())
    }

    pub async fn get_user_stats(&self, user_id: &str) -> anyhow::Result<UserStats> {
        let preferences = self.get_preferences(user_id).await?;

        let total_requests = preferences.interaction_history.len() as u32;
        let mut average_rating = 0.0;
        let mut preferred_models_usage = HashMap::new();
        let mut total_cost = 0.0;
        let mut total_time = 0;

        let mut ratings_sum = 0;
        let mut ratings_count = 0;

        for interaction in &preferences.interaction_history {
            // Count model usage
            *preferred_models_usage.entry(interaction.model_used.clone()).or_insert(0) += 1;

            // Sum ratings
            if let Some(rating) = interaction.rating {
                ratings_sum += rating as u32;
                ratings_count += 1;
            }

            // Sum cost and time
            if let Some(cost) = interaction.cost {
                total_cost += cost;
            }
            if let Some(time) = interaction.response_time_ms {
                total_time += time;
            }
        }

        if ratings_count > 0 {
            average_rating = ratings_sum as f32 / ratings_count as f32;
        }

        // Calculate savings compared to always using the most expensive model
        let baseline_cost = total_requests as f64 * 0.03; // Assuming GPT-4 as baseline
        let cost_savings = baseline_cost - total_cost;

        // Calculate time savings compared to slowest model
        let baseline_time = total_requests as u32 * 3000; // Assuming 3s as slow baseline
        let time_savings = baseline_time.saturating_sub(total_time);

        let learning_progress = preferences.calculate_personalization_strength() as f32;

        Ok(UserStats {
            user_id: user_id.to_string(),
            total_requests,
            average_rating,
            preferred_models_usage,
            cost_savings,
            time_savings_ms: time_savings,
            learning_progress,
        })
    }

    pub async fn delete_user_preferences(&self, user_id: &str) -> anyhow::Result<bool> {
        // Remove from cache
        {
            let mut cache = self.preferences_cache.lock().await;
            cache.remove(user_id);
        }

        // Remove from Redis
        if let Some(ref client) = self.redis_client {
            let mut conn = client.get_async_connection().await?;
            let key = format!("user_preferences:{}", user_id);

            match conn.del::<_, i32>(key).await {
                Ok(deleted) => Ok(deleted > 0),
                Err(e) => {
                    eprintln!("Error deleting from Redis: {}", e);
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }

    pub async fn get_all_users(&self) -> anyhow::Result<Vec<String>> {
        if let Some(ref client) = self.redis_client {
            let mut conn = client.get_async_connection().await?;

            // Get all keys matching user_preferences pattern
            let keys: Vec<String> = conn.keys("user_preferences:*").await?;

            // Extract user IDs from keys
            let user_ids: Vec<String> = keys.into_iter()
                .filter_map(|key| key.strip_prefix("user_preferences:").map(|s| s.to_string()))
                .collect();

            Ok(user_ids)
        } else {
            // Return cached user IDs if Redis is not available
            let cache = self.preferences_cache.lock().await;
            Ok(cache.keys().cloned().collect())
        }
    }

    pub async fn get_recommendations(&self, user_id: &str) -> anyhow::Result<Vec<String>> {
        let preferences = self.get_preferences(user_id).await?;
        let mut recommendations = Vec::new();

        // Analyze interaction patterns to make recommendations
        let mut model_ratings: HashMap<String, (u32, u32)> = HashMap::new(); // (total_rating, count)

        for interaction in &preferences.interaction_history {
            if let Some(rating) = interaction.rating {
                let entry = model_ratings.entry(interaction.model_used.clone()).or_insert((0, 0));
                entry.0 += rating as u32;
                entry.1 += 1;
            }
        }

        // Recommend models with high average ratings
        let mut model_scores: Vec<_> = model_ratings.iter()
            .filter_map(|(model, &(total, count))| {
                if count > 2 { // Need at least 3 interactions
                    Some((model, total as f64 / count as f64))
                } else {
                    None
                }
            })
            .collect();

        model_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for (model, score) in model_scores.iter().take(3) {
            if score >= &4.0 {
                recommendations.push(format!("Consider using {} more often (average rating: {:.1})", model, score));
            }
        }

        // Recommend optimization target changes
        if preferences.optimize_for != OptimizationTarget::Balanced {
            recommendations.push("Try balanced optimization for a mix of quality, speed, and cost".to_string());
        }

        // Recommend enabling learning if disabled
        if !preferences.learning_enabled {
            recommendations.push("Enable learning to let the system adapt to your preferences".to_string());
        }

        Ok(recommendations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_preference_manager_creation() {
        let manager = PreferenceManager::new().await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_get_default_preferences() {
        let manager = PreferenceManager::new().await.unwrap();
        let prefs = manager.get_preferences("test_user").await.unwrap();

        assert_eq!(prefs.user_id, "test_user");
        assert_eq!(prefs.optimize_for, OptimizationTarget::Balanced);
        assert!(prefs.learning_enabled);
    }

    #[tokio::test]
    async fn test_update_preferences() {
        let manager = PreferenceManager::new().await.unwrap();

        let update_request = PreferenceUpdateRequest {
            user_id: "test_user".to_string(),
            optimize_for: Some(OptimizationTarget::Quality),
            max_tokens: Some(4096),
            temperature: Some(0.8),
            custom_weights: None,
            preferred_models: None,
            excluded_models: None,
            learning_enabled: None,
        };

        let response = manager.update_preferences(update_request).await.unwrap();
        assert!(response.success);

        if let Some(prefs) = response.preferences {
            assert_eq!(prefs.optimize_for, OptimizationTarget::Quality);
            assert_eq!(prefs.max_tokens, 4096);
            assert_eq!(prefs.temperature, 0.8);
        }
    }

    #[tokio::test]
    async fn test_record_interaction() {
        let manager = PreferenceManager::new().await.unwrap();

        let interaction = UserInteraction {
            session_id: "test_session".to_string(),
            timestamp: chrono::Utc::now(),
            prompt_features: PromptInteractionFeatures {
                domain_category: "Technical".to_string(),
                task_type: "Question".to_string(),
                complexity_score: 0.7,
                estimated_tokens: 200,
                keywords: vec!["code".to_string()],
            },
            model_used: "gpt-4".to_string(),
            rating: Some(5),
            feedback_type: Some("Quality".to_string()),
            response_time_ms: Some(2000),
            cost: Some(0.03),
        };

        let result = manager.record_user_interaction("test_user", interaction).await;
        assert!(result.is_ok());

        // Check that interaction was recorded
        let prefs = manager.get_preferences("test_user").await.unwrap();
        assert!(!prefs.interaction_history.is_empty());
        assert_eq!(prefs.interaction_history[0].rating, Some(5));
    }

    #[tokio::test]
    async fn test_user_stats() {
        let manager = PreferenceManager::new().await.unwrap();

        // Add some interactions
        for i in 0..5 {
            let interaction = UserInteraction {
                session_id: format!("session_{}", i),
                timestamp: chrono::Utc::now(),
                prompt_features: PromptInteractionFeatures {
                    domain_category: "Technical".to_string(),
                    task_type: "Question".to_string(),
                    complexity_score: 0.5,
                    estimated_tokens: 150,
                    keywords: vec!["test".to_string()],
                },
                model_used: "gpt-4".to_string(),
                rating: Some(4),
                feedback_type: Some("Quality".to_string()),
                response_time_ms: Some(2000),
                cost: Some(0.02),
            };

            manager.record_user_interaction("stats_user", interaction).await.unwrap();
        }

        let stats = manager.get_user_stats("stats_user").await.unwrap();
        assert_eq!(stats.total_requests, 5);
        assert_eq!(stats.average_rating, 4.0);
        assert_eq!(stats.preferred_models_usage.get("gpt-4"), Some(&5));
        assert!(stats.cost_savings > 0.0);
    }
}