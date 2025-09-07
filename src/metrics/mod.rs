// src/metrics/mod.rs
use redis::AsyncCommands;

pub struct MetricCollector {
    conn: redis::aio::MultiplexedConnection,
}

impl MetricCollector {
    pub async fn connect() -> redis::RedisResult<Self> {
        let client = redis::Client::open("redis://127.0.0.1/")?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(MetricCollector { conn })
    }

    pub async fn record_success(&mut self, provider: &str, token_count: usize) {
        let key = format!("metrics:{}:success", provider);
        let timestamp = chrono::Utc::now().timestamp();
        let _: () = self
            .conn
            .zadd(key, format!("success-{}", timestamp), timestamp)
            .await
            .unwrap();

        let latency_key = format!("metrics:{}:tokens", provider);
        let _: () = self
            .conn
            .zadd(
                latency_key,
                format!("tokens-{}", timestamp),
                token_count as i32,
            )
            .await
            .unwrap();
    }

    pub async fn record_failure(&mut self, provider: &str) {
        let key = format!("metrics:{}:failure", provider);
        let timestamp = chrono::Utc::now().timestamp();
        let _: () = self
            .conn
            .zadd(key, format!("failure-{}", timestamp), timestamp)
            .await
            .unwrap();
    }
}
