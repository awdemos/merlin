// src/metrics/mod.rs
use redis::AsyncCommands;
use tracing::{error, warn};

/// Default Redis connection URL for metrics storage.
/// 
/// This can be overridden by passing a custom URL to `MetricCollector::connect_with_url()`.
/// The default assumes Redis is running locally on the standard port (6379).
const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1/";

/// How long metric entries are kept in Redis (7 days, in seconds).
/// Sorted sets are trimmed on every write so they cannot grow unbounded.
const METRIC_RETENTION_SECS: i64 = 7 * 24 * 60 * 60;

/// Collects and stores metrics in Redis for LLM provider performance tracking.
pub struct MetricCollector {
    /// `None` when Redis is unavailable: recording degrades to no-ops.
    conn: Option<redis::aio::MultiplexedConnection>,
    /// Monotonic counter to keep sorted-set members unique within a process,
    /// even for writes landing in the same nanosecond.
    write_counter: std::sync::atomic::AtomicU64,
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
        Ok(MetricCollector {
            conn: Some(conn),
            write_counter: std::sync::atomic::AtomicU64::new(0),
        })
    }

    /// Creates a degraded collector that silently drops all metrics.
    /// Used when Redis is unavailable so routing never panics on metrics.
    pub fn new_fallback() -> Self {
        MetricCollector {
            conn: None,
            write_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Builds a unique member id for a metric event. Previously the member was
    /// `format!("{kind}-{seconds}")`, so multiple events within the same second
    /// mapped to the same member and silently overwrote each other.
    fn unique_member(counter: &std::sync::atomic::AtomicU64, kind: &str, nanos: i64) -> String {
        let seq = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("{}-{}-{}", kind, nanos, seq)
    }

    /// Removes metric entries older than the retention window from a sorted set.
    async fn trim_old_entries(&mut self, key: &str, cutoff: i64) {
        let Some(conn) = self.conn.as_mut() else {
            return;
        };
        if let Err(e) = conn.zrembyscore::<_, _, _, ()>(key, 0, cutoff).await {
            warn!("Failed to trim old metrics for {}: {}", key, e);
        }
    }

    /// Records a successful request to a provider.
    ///
    /// # Arguments
    /// * `provider` - The name of the provider
    /// * `token_count` - The number of tokens used in the request
    pub async fn record_success(&mut self, provider: &str, token_count: usize) {
        let now = chrono::Utc::now();
        let timestamp = now.timestamp();
        let nanos = now.timestamp_nanos_opt().unwrap_or(timestamp);
        let cutoff = timestamp - METRIC_RETENTION_SECS;

        let success_member = Self::unique_member(&self.write_counter, "success", nanos);
        let tokens_member = Self::unique_member(&self.write_counter, "tokens", nanos);
        // i64 instead of i32: large token counts would overflow i32
        let token_count = token_count.min(i64::MAX as usize) as i64;

        let key = format!("metrics:{}:success", provider);
        let latency_key = format!("metrics:{}:tokens", provider);

        {
            let Some(conn) = self.conn.as_mut() else {
                return;
            };

            if let Err(e) = conn
                .zadd::<_, _, _, ()>(&key, success_member, timestamp)
                .await
            {
                error!("Failed to record success metric for {}: {}", provider, e);
            }

            if let Err(e) = conn
                .zadd::<_, _, _, ()>(&latency_key, tokens_member, token_count)
                .await
            {
                warn!("Failed to record token count for {}: {}", provider, e);
            }
        }

        self.trim_old_entries(&key, cutoff).await;
        self.trim_old_entries(&latency_key, cutoff).await;
    }

    /// Records a failed request to a provider.
    ///
    /// # Arguments
    /// * `provider` - The name of the provider
    pub async fn record_failure(&mut self, provider: &str) {
        let now = chrono::Utc::now();
        let timestamp = now.timestamp();
        let nanos = now.timestamp_nanos_opt().unwrap_or(timestamp);
        let member = Self::unique_member(&self.write_counter, "failure", nanos);

        let key = format!("metrics:{}:failure", provider);
        {
            let Some(conn) = self.conn.as_mut() else {
                return;
            };
            if let Err(e) = conn
                .zadd::<_, _, _, ()>(&key, member, timestamp)
                .await
            {
                error!("Failed to record failure metric for {}: {}", provider, e);
            }
        }
        self.trim_old_entries(&key, timestamp - METRIC_RETENTION_SECS).await;
    }
}
