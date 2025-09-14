// src/feedback/processor.rs
use crate::api::{FeedbackRequest, FeedbackType};
use crate::feedback::storage::{FeedbackStorage, ModelFeedbackStats};
use crate::routing::RoutingPolicy;

pub struct FeedbackProcessor {
    storage: FeedbackStorage,
}

impl FeedbackProcessor {
    pub async fn new() -> anyhow::Result<Self> {
        let storage = FeedbackStorage::connect().await?;
        Ok(FeedbackProcessor { storage })
    }

    pub async fn process_feedback(&mut self, feedback: &FeedbackRequest) -> anyhow::Result<()> {
        // Store the feedback
        self.storage.store_feedback(feedback).await?;
        
        // Process feedback for learning (could be expanded with ML models)
        self.update_quality_metrics(feedback).await?;
        
        Ok(())
    }

    pub async fn get_model_performance(&mut self, model_name: &str) -> anyhow::Result<ModelFeedbackStats> {
        self.storage.get_model_feedback_stats(model_name).await
    }

    pub async fn calculate_reward_score(&mut self, session_id: &str) -> anyhow::Result<Option<f64>> {
        if let Some(feedback) = self.storage.get_feedback(session_id).await? {
            let reward_score = self.feedback_to_reward_score(&feedback);
            Ok(Some(reward_score))
        } else {
            Ok(None)
        }
    }

    pub async fn update_routing_policy_from_feedback(
        &mut self,
        routing_policy: &mut RoutingPolicy,
        model_index_map: &std::collections::HashMap<String, usize>,
    ) -> anyhow::Result<u32> {
        let mut updates_count = 0;
        
        // Get feedback for all models and update routing policy
        for (model_name, &provider_index) in model_index_map {
            let recent_feedback = self.storage
                .get_recent_feedback_for_model(model_name, 50)
                .await?;
            
            for feedback in recent_feedback {
                let reward_score = self.feedback_to_reward_score(&feedback);
                routing_policy.update_reward_with_score(provider_index, reward_score);
                updates_count += 1;
            }
        }

        Ok(updates_count)
    }

    fn feedback_to_reward_score(&self, feedback: &crate::feedback::storage::StoredFeedback) -> f64 {
        // Convert feedback to a reward score between 0.0 and 1.0
        let base_score = (feedback.rating as f64 - 1.0) / 4.0; // Scale 1-5 to 0-1
        
        // Adjust based on feedback type
        let type_weight = match feedback.feedback_type {
            FeedbackType::Quality => 1.0,    // Quality feedback gets full weight
            FeedbackType::Speed => 0.8,      // Speed feedback slightly less weight
            FeedbackType::Cost => 0.6,       // Cost feedback lower weight for quality assessment
            FeedbackType::Overall => 1.0,    // Overall feedback gets full weight
        };

        // Apply sentiment analysis to comments (simple keyword-based)
        let comment_modifier = if let Some(comment) = &feedback.comment {
            self.analyze_comment_sentiment(comment)
        } else {
            0.0 // Neutral if no comment
        };

        let final_score = (base_score * type_weight + comment_modifier * 0.1).clamp(0.0, 1.0);
        final_score
    }

    fn analyze_comment_sentiment(&self, comment: &str) -> f64 {
        let comment_lower = comment.to_lowercase();
        let mut sentiment_score: f64 = 0.0;

        // Positive indicators
        let positive_words = [
            "excellent", "great", "good", "amazing", "perfect", "helpful",
            "accurate", "fast", "efficient", "love", "best", "awesome"
        ];
        
        // Negative indicators
        let negative_words = [
            "terrible", "bad", "awful", "wrong", "slow", "useless",
            "inaccurate", "poor", "disappointing", "hate", "worst", "broken"
        ];

        for word in positive_words.iter() {
            if comment_lower.contains(word) {
                sentiment_score += 0.2;
            }
        }

        for word in negative_words.iter() {
            if comment_lower.contains(word) {
                sentiment_score -= 0.2;
            }
        }

        sentiment_score.clamp(-1.0, 1.0)
    }

    async fn update_quality_metrics(&mut self, _feedback: &FeedbackRequest) -> anyhow::Result<()> {
        // This could be expanded to update more sophisticated quality metrics
        // For now, just store the feedback which is handled by the storage layer
        
        // Future enhancements could include:
        // - Updating model quality scores
        // - Triggering retraining of routing models
        // - Sending alerts for poor performance
        // - Updating user preference models
        
        Ok(())
    }

    pub fn new_fallback() -> Self {
        Self {
            storage: FeedbackStorage::new_fallback(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{FeedbackRequest, FeedbackType};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_feedback_to_reward_score() {
        let processor = match FeedbackProcessor::new().await {
            Ok(processor) => processor,
            Err(_) => return, // Skip test if Redis not available
        };

        // Test high rating
        let high_rating_feedback = crate::feedback::storage::StoredFeedback {
            session_id: "test".to_string(),
            model_used: "test".to_string(),
            rating: 5,
            feedback_type: FeedbackType::Quality,
            comment: Some("Excellent response!".to_string()),
            metadata: None,
            timestamp: 0,
        };

        let score = processor.feedback_to_reward_score(&high_rating_feedback);
        assert!(score > 0.9, "High rating should produce high reward score");

        // Test low rating
        let low_rating_feedback = crate::feedback::storage::StoredFeedback {
            session_id: "test".to_string(),
            model_used: "test".to_string(),
            rating: 1,
            feedback_type: FeedbackType::Quality,
            comment: Some("Terrible response!".to_string()),
            metadata: None,
            timestamp: 0,
        };

        let score = processor.feedback_to_reward_score(&low_rating_feedback);
        assert!(score < 0.1, "Low rating should produce low reward score");
    }

    #[tokio::test]
    async fn test_comment_sentiment_analysis() {
        // Create a proper processor instance or skip if Redis is not available
        let processor = match FeedbackProcessor::new().await {
            Ok(processor) => processor,
            Err(_) => return, // Skip test if Redis not available
        };

        assert!(processor.analyze_comment_sentiment("This is excellent!") > 0.0);
        assert!(processor.analyze_comment_sentiment("This is terrible!") < 0.0);
        assert!(processor.analyze_comment_sentiment("This is okay.") == 0.0);
    }
}
