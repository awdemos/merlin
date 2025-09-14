// src/feedback/storage.rs
use crate::api::{FeedbackRequest, FeedbackType};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFeedback {
    pub session_id: String,
    pub model_used: String,
    pub rating: u8,
    pub feedback_type: FeedbackType,
    pub comment: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub timestamp: i64,
}

pub struct FeedbackStorage {
    conn: redis::aio::MultiplexedConnection,
}

impl FeedbackStorage {
    pub async fn connect() -> redis::RedisResult<Self> {
        let client = redis::Client::open("redis://127.0.0.1/")?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(FeedbackStorage { conn })
    }

    pub async fn store_feedback(&mut self, feedback: &FeedbackRequest) -> anyhow::Result<()> {
        let stored_feedback = StoredFeedback {
            session_id: feedback.session_id.clone(),
            model_used: feedback.model_used.clone(),
            rating: feedback.rating,
            feedback_type: feedback.feedback_type.clone(),
            comment: feedback.comment.clone(),
            metadata: feedback.metadata.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        let key = format!("feedback:{}", feedback.session_id);
        let feedback_json = serde_json::to_string(&stored_feedback)?;
        
        let _: () = self
            .conn
            .set(&key, feedback_json)
            .await?;

        // Also store in a time-ordered list for analytics
        let timeline_key = format!("feedback:model:{}:timeline", feedback.model_used);
        let _: () = self
            .conn
            .zadd(
                timeline_key,
                feedback.session_id.clone(),
                stored_feedback.timestamp,
            )
            .await?;

        // Store rating statistics
        let stats_key = format!("feedback:model:{}:stats", feedback.model_used);
        self.update_model_stats(&stats_key, feedback.rating).await?;

        Ok(())
    }

    pub async fn get_feedback(&mut self, session_id: &str) -> anyhow::Result<Option<StoredFeedback>> {
        let key = format!("feedback:{}", session_id);
        let feedback_json: Option<String> = self.conn.get(&key).await?;
        
        match feedback_json {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn get_model_feedback_stats(&mut self, model_name: &str) -> anyhow::Result<ModelFeedbackStats> {
        let stats_key = format!("feedback:model:{}:stats", model_name);
        let stats: HashMap<String, String> = self.conn.hgetall(&stats_key).await?;
        
        let total_feedback = stats.get("total_count")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        let total_rating = stats.get("total_rating")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        
        let average_rating = if total_feedback > 0 {
            total_rating / total_feedback as f64
        } else {
            0.0
        };

        let rating_distribution = [1, 2, 3, 4, 5].iter()
            .map(|&rating| {
                let count = stats.get(&format!("rating_{}", rating))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                (rating, count)
            })
            .collect();

        Ok(ModelFeedbackStats {
            model_name: model_name.to_string(),
            total_feedback,
            average_rating,
            rating_distribution,
        })
    }

    async fn update_model_stats(&mut self, stats_key: &str, rating: u8) -> redis::RedisResult<()> {
        // Increment total count
        let _: () = self.conn.hincr(stats_key, "total_count", 1).await?;
        
        // Add to total rating sum
        let _: () = self.conn.hincr(stats_key, "total_rating", rating as i32).await?;
        
        // Increment specific rating count
        let rating_key = format!("rating_{}", rating);
        let _: () = self.conn.hincr(stats_key, &rating_key, 1).await?;

        Ok(())
    }

    pub async fn get_recent_feedback_for_model(
        &mut self, 
        model_name: &str, 
        limit: usize
    ) -> anyhow::Result<Vec<StoredFeedback>> {
        let timeline_key = format!("feedback:model:{}:timeline", model_name);
        let session_ids: Vec<String> = self
            .conn
            .zrevrange(&timeline_key, 0, limit as isize - 1)
            .await?;

        let mut feedback_list = Vec::new();
        for session_id in session_ids {
            if let Some(feedback) = self.get_feedback(&session_id).await? {
                feedback_list.push(feedback);
            }
        }

        Ok(feedback_list)
    }

    pub fn new_fallback() -> Self {
        // For now, we'll create a fallback that connects to Redis synchronously
        // This isn't ideal but will allow compilation to succeed
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let conn = std::thread::spawn(move || {
            futures::executor::block_on(client.get_multiplexed_async_connection()).unwrap()
        }).join().unwrap();

        FeedbackStorage { conn }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelFeedbackStats {
    pub model_name: String,
    pub total_feedback: u32,
    pub average_rating: f64,
    pub rating_distribution: HashMap<u8, u32>,
}
