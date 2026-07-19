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
    /// `None` when Redis is unavailable: all operations degrade to no-ops.
    conn: Option<redis::aio::MultiplexedConnection>,
    /// High-water marks of feedback timestamps already applied to routing
    /// policies, per model. Prevents double-counting rewards on replay.
    applied_watermarks: HashMap<String, i64>,
}

impl FeedbackStorage {
    pub async fn connect() -> redis::RedisResult<Self> {
        let client = redis::Client::open("redis://127.0.0.1/")?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(FeedbackStorage {
            conn: Some(conn),
            applied_watermarks: HashMap::new(),
        })
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

        let Some(conn) = self.conn.as_mut() else {
            // Fallback storage without Redis: drop the feedback silently.
            return Ok(());
        };

        let feedback_json = serde_json::to_string(&stored_feedback)?;

        // Store every feedback event under a unique key so repeat feedback for
        // the same session accumulates instead of being overwritten.
        let nanos = chrono::Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or(stored_feedback.timestamp);
        let event_key = format!("feedback:{}:{}", feedback.session_id, nanos);
        let _: () = conn.set(&event_key, &feedback_json).await?;

        // Keep a "latest feedback for session" pointer for get_feedback()
        let latest_key = format!("feedback:{}", feedback.session_id);
        let _: () = conn.set(&latest_key, &feedback_json).await?;

        // Also store in a time-ordered list for analytics. The member is the
        // unique event key so multiple events per session are all retained.
        let timeline_key = format!("feedback:model:{}:timeline", feedback.model_used);
        let _: () = conn
            .zadd(
                timeline_key,
                event_key,
                stored_feedback.timestamp,
            )
            .await?;

        // Store rating statistics
        let stats_key = format!("feedback:model:{}:stats", feedback.model_used);
        Self::update_model_stats(conn, &stats_key, feedback.rating).await?;

        Ok(())
    }

    pub async fn get_feedback(&mut self, session_id: &str) -> anyhow::Result<Option<StoredFeedback>> {
        let Some(conn) = self.conn.as_mut() else {
            return Ok(None);
        };
        let key = format!("feedback:{}", session_id);
        let feedback_json: Option<String> = conn.get(&key).await?;
        
        match feedback_json {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn get_model_feedback_stats(&mut self, model_name: &str) -> anyhow::Result<ModelFeedbackStats> {
        let Some(conn) = self.conn.as_mut() else {
            return Ok(ModelFeedbackStats {
                model_name: model_name.to_string(),
                total_feedback: 0,
                average_rating: 0.0,
                rating_distribution: HashMap::new(),
            });
        };
        let stats_key = format!("feedback:model:{}:stats", model_name);
        let stats: HashMap<String, String> = conn.hgetall(&stats_key).await?;
        
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

    async fn update_model_stats(conn: &mut redis::aio::MultiplexedConnection, stats_key: &str, rating: u8) -> redis::RedisResult<()> {
        // Increment total count
        let _: () = conn.hincr(stats_key, "total_count", 1).await?;
        
        // Add to total rating sum
        let _: () = conn.hincr(stats_key, "total_rating", rating as i32).await?;
        
        // Increment specific rating count
        let rating_key = format!("rating_{}", rating);
        let _: () = conn.hincr(stats_key, &rating_key, 1).await?;

        Ok(())
    }

    pub async fn get_recent_feedback_for_model(
        &mut self, 
        model_name: &str, 
        limit: usize
    ) -> anyhow::Result<Vec<StoredFeedback>> {
        let Some(conn) = self.conn.as_mut() else {
            return Ok(Vec::new());
        };
        let timeline_key = format!("feedback:model:{}:timeline", model_name);
        let members: Vec<String> = conn
            .zrevrange(&timeline_key, 0, limit as isize - 1)
            .await?;

        let mut feedback_list = Vec::new();
        for member in members {
            // New entries use the full event key as member; legacy entries
            // used the bare session id (keyed as feedback:{session_id}).
            let key = if member.starts_with("feedback:") {
                member
            } else {
                format!("feedback:{}", member)
            };
            let feedback_json: Option<String> = conn.get(&key).await?;
            if let Some(json) = feedback_json {
                feedback_list.push(serde_json::from_str(&json)?);
            }
        }

        Ok(feedback_list)
    }

    /// Returns feedback for a model that has not yet been applied to routing
    /// policy updates (strictly newer than the recorded watermark), oldest
    /// first, and advances the watermark past everything returned so rewards
    /// are never applied twice.
    pub async fn get_unapplied_feedback_for_model(
        &mut self,
        model_name: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<StoredFeedback>> {
        let watermark = self.get_applied_watermark(model_name).await;

        // Newest first from the timeline
        let mut recent = self.get_recent_feedback_for_model(model_name, limit).await?;
        recent.retain(|f| f.timestamp > watermark);
        // Apply chronologically so learning sees rewards in order
        recent.reverse();

        if let Some(max_ts) = recent.iter().map(|f| f.timestamp).max() {
            self.set_applied_watermark(model_name, max_ts).await;
        }

        Ok(recent)
    }

    /// Reads the applied watermark for a model (in-memory first, then Redis).
    async fn get_applied_watermark(&mut self, model_name: &str) -> i64 {
        if let Some(&ts) = self.applied_watermarks.get(model_name) {
            return ts;
        }
        if let Some(conn) = self.conn.as_mut() {
            let key = format!("feedback:model:{}:reward_watermark", model_name);
            if let Ok(Some(ts)) = conn.get::<_, Option<i64>>(&key).await {
                self.applied_watermarks.insert(model_name.to_string(), ts);
                return ts;
            }
        }
        0
    }

    /// Persists the applied watermark for a model (in-memory + Redis best effort).
    async fn set_applied_watermark(&mut self, model_name: &str, ts: i64) {
        self.applied_watermarks.insert(model_name.to_string(), ts);
        if let Some(conn) = self.conn.as_mut() {
            let key = format!("feedback:model:{}:reward_watermark", model_name);
            let _: redis::RedisResult<()> = conn.set(&key, ts).await;
        }
    }

    /// Creates a degraded storage that silently drops writes and returns
    /// empty results for reads. Used when Redis is unavailable so that
    /// feedback processing never panics or blocks request handling.
    pub fn new_fallback() -> Self {
        FeedbackStorage {
            conn: None,
            applied_watermarks: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelFeedbackStats {
    pub model_name: String,
    pub total_feedback: u32,
    pub average_rating: f64,
    pub rating_distribution: HashMap<u8, u32>,
}
