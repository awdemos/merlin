// src/metrics/mod.rs
use redis::AsyncCommands;
use tracing::{error, warn};

/// Default Redis connection URL for metrics storage.
/// 
/// This can be overridden by passing a custom URL to `MetricCollector::connect_with_url()`.
/// The default assumes Redis is running locally on the standard port (6379).
const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1/";

/// Collects and stores metrics in Redis for LLM provider performance tracking.
pub struct MetricCollector {
    conn: redis::aio::MultiplexedConnection,
}

impl MetricCollector {
    /// Creates a new MetricCollector by connecting to Redis using the default URL.
    ///
    /// Uses `DEFAULT_REDIS_URL` ("redis://127.0.0.1/") as the connection string.
    /// For custom Redis configurations, use `connect_with_url()`.
    ///
    /// # Errors
    /// Returns an error if the connection to Redis fails.
    pub async fn connect() -> redis::RedisResult<Self> {
        Self::connect_with_url(DEFAULT_REDIS_URL).await
    }

    /// Creates a new MetricCollector by connecting to a custom Redis URL.
    ///
    /// # Arguments
    /// * `url` - The Redis connection URL (e.g., "redis://localhost:6379/" or "redis://user:pass@host/")
    ///
    /// # Errors
    /// Returns an error if the connection to Redis fails.
    pub async fn connect_with_url(url: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(MetricCollector { conn })
    }

    /// Records a successful request to a provider.
    ///
    /// # Arguments
    /// * `provider` - The name of the provider
    /// * `token_count` - The number of tokens used in the request
    pub async fn record_success(&mut self, provider: &str, token_count: usize) {
        let key = format!("metrics:{}:success", provider);
        let timestamp = chrono::Utc::now().timestamp();
        
        if let Err(e) = self
            .conn
            .zadd::<_, _, _, ()>(&key, format!("success-{}", timestamp), timestamp)
            .await
        {
            error!("Failed to record success metric for {}: {}", provider, e);
        }

        let latency_key = format!("metrics:{}:tokens", provider);
        if let Err(e) = self
            .conn
            .zadd::<_, _, _, ()>(&latency_key, format!("tokens-{}", timestamp), token_count as i32)
            .await
        {
            warn!("Failed to record token count for {}: {}", provider, e);
        }
    }

    /// Records a failed request to a provider.
    ///
    /// # Arguments
    /// * `provider` - The name of the provider
    pub async fn record_failure(&mut self, provider: &str) {
        let key = format!("metrics:{}:failure", provider);
        let timestamp = chrono::Utc::now().timestamp();
        
        if let Err(e) = self
            .conn
            .zadd::<_, _, _, ()>(&key, format!("failure-{}", timestamp), timestamp)
            .await
        {
            error!("Failed to record failure metric for {}: {}", provider, e);
        }
    }
}
