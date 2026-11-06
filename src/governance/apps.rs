//! App identity, policy, and token storage for the governance suite.
//!
//! Tokens are random 256-bit secrets presented to callers exactly once
//! at issue; only the SHA-256 hash is stored, so a store dump yields no
//! usable credentials. Usage counters (per-minute RPM, per-day spend)
//! live in the same store behind the [`Store`] trait.

use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Whether an app's traffic may leave local infrastructure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LocalityPolicy {
    #[default]
    Any,
    LocalOnly,
}

/// Per-app policy enforced by the gate and quotas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPolicy {
    /// Client-visible route allowlist. `None` = all routes.
    pub allowed_routes: Option<Vec<String>>,
    /// Upstream target allowlist. `None` = all targets.
    pub allowed_targets: Option<Vec<String>>,
    pub locality: LocalityPolicy,
    /// Requests per minute; `None` = unlimited.
    pub max_rpm: Option<u32>,
    /// Daily spend cap in USD; `None` = unlimited. Charged on aggregate
    /// responses and, via accumulated usage events, on streamed responses.
    pub max_spend_usd_per_day: Option<f64>,
    /// Raise-only: sensitive requests are restricted to local targets.
    pub sensitive_local_only: bool,
}

/// A registered calling application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct App {
    pub id: String,
    pub name: String,
    pub policy: AppPolicy,
    pub created_at: chrono::DateTime<Utc>,
}

impl App {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            policy: AppPolicy::default(),
            created_at: Utc::now(),
        }
    }
}

/// Mint a new raw API token. The caller must display it once and then
/// store only [`hash_token`] of it.
pub fn mint_token() -> String {
    let mut bytes = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut bytes);
    format!(
        "mrl_live_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

/// SHA-256 of the raw token, hex-encoded. This is what gets stored.
pub fn hash_token(raw: &str) -> String {
    let digest = sha2::Sha256::digest(raw.as_bytes());
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Errors from store operations. The server maps these to 503.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("store backend error: {0}")]
    Backend(String),
    #[error("app not found: {0}")]
    NotFound(String),
}

/// Persistence for apps, tokens, and usage counters.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait Store: Send + Sync {
    async fn put_app(&self, app: &App) -> Result<(), StoreError>;
    async fn get_app(&self, id: &str) -> Result<Option<App>, StoreError>;
    async fn list_apps(&self) -> Result<Vec<App>, StoreError>;

    /// Record a token hash under `token_id` for `app_id`.
    async fn put_token(&self, app_id: &str, token_id: &str, hash: &str) -> Result<(), StoreError>;
    /// Remove a token. Revocation deletes the lookup, so the token
    /// stops authenticating immediately.
    async fn remove_token(&self, app_id: &str, token_id: &str) -> Result<(), StoreError>;
    /// Resolve a token hash to `(app_id, token_id)`.
    async fn app_for_token(&self, hash: &str) -> Result<Option<(String, String)>, StoreError>;
    async fn token_ids(&self, app_id: &str) -> Result<Vec<String>, StoreError>;

    /// Increment the per-minute request counter; returns the new count.
    async fn incr_rpm(&self, app_id: &str, minute_epoch: i64) -> Result<u64, StoreError>;
    /// Current-day spend in USD.
    async fn spend(&self, app_id: &str, day: &str) -> Result<f64, StoreError>;
    /// Add to today's spend; returns the new total.
    async fn add_spend(&self, app_id: &str, day: &str, amount: f64) -> Result<f64, StoreError>;
}

/// Store used when governance is enabled but the backend is unreachable.
/// Every operation fails, so governed requests fail closed (503) and
/// token-present auth never silently passes.
pub struct UnavailableStore;

#[async_trait]
impl Store for UnavailableStore {
    async fn put_app(&self, _app: &App) -> Result<(), StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }

    async fn get_app(&self, _id: &str) -> Result<Option<App>, StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }

    async fn list_apps(&self) -> Result<Vec<App>, StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }

    async fn put_token(
        &self,
        _app_id: &str,
        _token_id: &str,
        _hash: &str,
    ) -> Result<(), StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }

    async fn remove_token(&self, _app_id: &str, _token_id: &str) -> Result<(), StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }

    async fn app_for_token(&self, _hash: &str) -> Result<Option<(String, String)>, StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }

    async fn token_ids(&self, _app_id: &str) -> Result<Vec<String>, StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }

    async fn incr_rpm(&self, _app_id: &str, _minute_epoch: i64) -> Result<u64, StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }

    async fn spend(&self, _app_id: &str, _day: &str) -> Result<f64, StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }

    async fn add_spend(&self, _app_id: &str, _day: &str, _amount: f64) -> Result<f64, StoreError> {
        Err(StoreError::Backend("governance store unavailable".into()))
    }
}

/// In-memory store for tests and single-node deployments.
#[derive(Default)]
pub struct MemoryStore {
    inner: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    apps: HashMap<String, App>,
    /// app_id -> (token_id -> hash)
    tokens: HashMap<String, HashMap<String, String>>,
    /// hash -> (app_id, token_id)
    token_index: HashMap<String, (String, String)>,
    /// (app_id, minute_epoch) -> count
    rpm: HashMap<(String, i64), u64>,
    /// (app_id, day) -> spend
    spend: HashMap<(String, String), f64>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn put_app(&self, app: &App) -> Result<(), StoreError> {
        self.inner
            .write()
            .await
            .apps
            .insert(app.id.clone(), app.clone());
        Ok(())
    }

    async fn get_app(&self, id: &str) -> Result<Option<App>, StoreError> {
        Ok(self.inner.read().await.apps.get(id).cloned())
    }

    async fn list_apps(&self) -> Result<Vec<App>, StoreError> {
        Ok(self.inner.read().await.apps.values().cloned().collect())
    }

    async fn put_token(&self, app_id: &str, token_id: &str, hash: &str) -> Result<(), StoreError> {
        let mut inner = self.inner.write().await;
        if let Some(old_hash) = inner.tokens.get(app_id).and_then(|m| m.get(token_id)) {
            let old_hash = old_hash.clone();
            inner.token_index.remove(&old_hash);
        }
        inner
            .tokens
            .entry(app_id.to_string())
            .or_default()
            .insert(token_id.to_string(), hash.to_string());
        inner
            .token_index
            .insert(hash.to_string(), (app_id.to_string(), token_id.to_string()));
        Ok(())
    }

    async fn remove_token(&self, app_id: &str, token_id: &str) -> Result<(), StoreError> {
        let mut inner = self.inner.write().await;
        if let Some(hash) = inner
            .tokens
            .get_mut(app_id)
            .and_then(|tokens| tokens.remove(token_id))
        {
            inner.token_index.remove(&hash);
        }
        Ok(())
    }

    async fn app_for_token(&self, hash: &str) -> Result<Option<(String, String)>, StoreError> {
        Ok(self.inner.read().await.token_index.get(hash).cloned())
    }

    async fn token_ids(&self, app_id: &str) -> Result<Vec<String>, StoreError> {
        Ok(self
            .inner
            .read()
            .await
            .tokens
            .get(app_id)
            .map(|tokens| tokens.keys().cloned().collect())
            .unwrap_or_default())
    }

    async fn incr_rpm(&self, app_id: &str, minute_epoch: i64) -> Result<u64, StoreError> {
        let mut inner = self.inner.write().await;
        let count = inner
            .rpm
            .entry((app_id.to_string(), minute_epoch))
            .or_insert(0);
        *count += 1;
        Ok(*count)
    }

    async fn spend(&self, app_id: &str, day: &str) -> Result<f64, StoreError> {
        Ok(self
            .inner
            .read()
            .await
            .spend
            .get(&(app_id.to_string(), day.to_string()))
            .copied()
            .unwrap_or(0.0))
    }

    async fn add_spend(&self, app_id: &str, day: &str, amount: f64) -> Result<f64, StoreError> {
        let mut inner = self.inner.write().await;
        let total = inner
            .spend
            .entry((app_id.to_string(), day.to_string()))
            .or_insert(0.0);
        *total += amount;
        Ok(*total)
    }
}

/// Redis-backed store. Keys:
///
/// ```text
/// gvn:app:<id>                 hash    {json: <App JSON>}
/// gvn:apps                     set     app ids (SADD by put_app/put_token; index for list_apps)
/// gvn:app:<id>:tokens          hash    {token_id: sha256}
/// gvn:tok:<sha256>             hash    {app: <id>, tid: <token_id>}
/// gvn:app:<id>:rpm:<minute>    string  INCR, EXPIRE 90
/// gvn:app:<id>:spend:<day>     string  INCRBYFLOAT, EXPIRE 172800
/// ```
pub struct RedisStore {
    conn: redis::aio::ConnectionManager,
}

/// Timeout for establishing a Redis connection.
const REDIS_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
/// Timeout for an individual Redis command response.
const REDIS_RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

impl RedisStore {
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let client = redis::Client::open(url)
            .map_err(|e| StoreError::Backend(format!("bad redis url: {e}")))?;
        // redis 0.25 has no ConnectionManagerConfig; this is the only
        // constructor that takes timeouts. Backoff args mirror the crate's
        // private defaults used by ConnectionManager::new.
        let conn = redis::aio::ConnectionManager::new_with_backoff_and_timeouts(
            client,
            2,
            100,
            6,
            REDIS_RESPONSE_TIMEOUT,
            REDIS_CONNECT_TIMEOUT,
        )
        .await
        .map_err(|e| StoreError::Backend(format!("redis connect failed: {e}")))?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl Store for RedisStore {
    async fn put_app(&self, app: &App) -> Result<(), StoreError> {
        use redis::AsyncCommands;
        let json = serde_json::to_string(app)
            .map_err(|e| StoreError::Backend(format!("serialize app: {e}")))?;
        let mut conn = self.conn.clone();
        // SADD first: crash-ordering prefers listed-but-missing over
        // missing-but-authenticating.
        conn.sadd::<_, _, ()>("gvn:apps", &app.id)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        conn.hset::<_, _, _, ()>(format!("gvn:app:{}", app.id), "json", json)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))
    }

    async fn get_app(&self, id: &str) -> Result<Option<App>, StoreError> {
        use redis::AsyncCommands;
        let json: Option<String> = self
            .conn
            .clone()
            .hget(format!("gvn:app:{id}"), "json")
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        match json {
            None => Ok(None),
            Some(j) => serde_json::from_str(&j)
                .map(Some)
                .map_err(|e| StoreError::Backend(format!("deserialize app: {e}"))),
        }
    }

    async fn list_apps(&self) -> Result<Vec<App>, StoreError> {
        use redis::AsyncCommands;
        let ids: Vec<String> = self
            .conn
            .clone()
            .smembers("gvn:apps")
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let mut apps = Vec::new();
        for id in ids {
            if let Some(app) = self.get_app(&id).await? {
                apps.push(app);
            }
        }
        Ok(apps)
    }

    /// Record a token hash under `token_id` for `app_id`. Re-issue runs as
    /// one atomic Lua script: it evicts any old reverse-index entry before
    /// writing the forward and reverse mappings, so a superseded token
    /// stops authenticating immediately and concurrent re-issues cannot
    /// leave a ghost entry — mirroring [`MemoryStore::put_token`].
    async fn put_token(&self, app_id: &str, token_id: &str, hash: &str) -> Result<(), StoreError> {
        use redis::AsyncCommands;
        let mut conn = self.conn.clone();
        conn.sadd::<_, _, ()>("gvn:apps", app_id)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let script = redis::Script::new(
            r#"
            local old = redis.call('HGET', KEYS[1], ARGV[1])
            if old and old ~= ARGV[2] then
                redis.call('DEL', 'gvn:tok:' .. old)
            end
            redis.call('HSET', KEYS[1], ARGV[1], ARGV[2])
            redis.call('HSET', 'gvn:tok:' .. ARGV[2], 'app', ARGV[3], 'tid', ARGV[1])
            return 1
            "#,
        );
        script
            .key(format!("gvn:app:{app_id}:tokens"))
            .arg(token_id)
            .arg(hash)
            .arg(app_id)
            .invoke_async::<_, i64>(&mut conn)
            .await
            .map(|_| ())
            .map_err(|e| StoreError::Backend(e.to_string()))
    }

    /// Remove a token. One atomic Lua script: read the stored hash, delete
    /// the reverse-index entry, then drop the forward entry — a crash
    /// mid-revoke cannot leave a live reverse index for a revoked token.
    async fn remove_token(&self, app_id: &str, token_id: &str) -> Result<(), StoreError> {
        let mut conn = self.conn.clone();
        let script = redis::Script::new(
            r#"
            local hash = redis.call('HGET', KEYS[1], ARGV[1])
            if hash then
                redis.call('DEL', 'gvn:tok:' .. hash)
            end
            redis.call('HDEL', KEYS[1], ARGV[1])
            return 1
            "#,
        );
        script
            .key(format!("gvn:app:{app_id}:tokens"))
            .arg(token_id)
            .invoke_async::<_, i64>(&mut conn)
            .await
            .map(|_| ())
            .map_err(|e| StoreError::Backend(e.to_string()))
    }

    async fn app_for_token(&self, hash: &str) -> Result<Option<(String, String)>, StoreError> {
        use redis::AsyncCommands;
        let fields: std::collections::HashMap<String, String> = self
            .conn
            .clone()
            .hgetall(format!("gvn:tok:{hash}"))
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        match (fields.get("app"), fields.get("tid")) {
            (Some(app), Some(tid)) => Ok(Some((app.clone(), tid.clone()))),
            _ => Ok(None),
        }
    }

    async fn token_ids(&self, app_id: &str) -> Result<Vec<String>, StoreError> {
        use redis::AsyncCommands;
        self.conn
            .clone()
            .hkeys(format!("gvn:app:{app_id}:tokens"))
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))
    }

    async fn incr_rpm(&self, app_id: &str, minute_epoch: i64) -> Result<u64, StoreError> {
        use redis::AsyncCommands;
        let key = format!("gvn:app:{app_id}:rpm:{minute_epoch}");
        let count: i64 = self
            .conn
            .clone()
            .incr(&key, 1)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        if count == 1 {
            if let Err(e) = self.conn.clone().expire::<_, i64>(&key, 90).await {
                tracing::warn!(key = %key, error = %e, "failed to set TTL on rpm counter");
            }
        }
        Ok(count as u64)
    }

    async fn spend(&self, app_id: &str, day: &str) -> Result<f64, StoreError> {
        use redis::AsyncCommands;
        let value: Option<String> = self
            .conn
            .clone()
            .get(format!("gvn:app:{app_id}:spend:{day}"))
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        match value {
            Some(v) => v
                .parse::<f64>()
                .map_err(|e| StoreError::Backend(e.to_string())),
            None => Ok(0.0),
        }
    }

    async fn add_spend(&self, app_id: &str, day: &str, amount: f64) -> Result<f64, StoreError> {
        use redis::AsyncCommands;
        let key = format!("gvn:app:{app_id}:spend:{day}");
        let mut conn = self.conn.clone();
        let total: f64 = conn
            .incr(&key, amount)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        // refresh TTL; key exists post-increment
        if let Err(e) = conn.expire::<_, i64>(&key, 172_800).await {
            tracing::warn!(key = %key, error = %e, "failed to refresh TTL on spend counter");
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_get_list_app() {
        let store = MemoryStore::new();
        let app = App::new("myapp", "My App");
        store.put_app(&app).await.unwrap();
        let fetched = store.get_app("myapp").await.unwrap().expect("app exists");
        assert_eq!(fetched.name, "My App");
        assert_eq!(fetched.policy, AppPolicy::default());
        assert_eq!(store.list_apps().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn minted_token_authenticates_then_revoke_denies() {
        let store = MemoryStore::new();
        let app = App::new("myapp", "My App");
        store.put_app(&app).await.unwrap();

        let raw = mint_token();
        let hash = hash_token(&raw);
        let token_id = "tid-1".to_string();
        store.put_token("myapp", &token_id, &hash).await.unwrap();

        let (app_id, stored_tid) = store
            .app_for_token(&hash)
            .await
            .unwrap()
            .expect("token resolves");
        assert_eq!(app_id, "myapp");
        assert_eq!(stored_tid, token_id);
        assert_eq!(store.token_ids("myapp").await.unwrap(), vec!["tid-1"]);

        store.remove_token("myapp", &token_id).await.unwrap();
        assert!(store.app_for_token(&hash).await.unwrap().is_none());
        assert!(store.token_ids("myapp").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn reissue_invalidates_old_hash_and_revokes_both() {
        let store = MemoryStore::new();
        let app = App::new("myapp", "My App");
        store.put_app(&app).await.unwrap();

        let raw1 = mint_token();
        let hash1 = hash_token(&raw1);
        store.put_token("myapp", "tid-1", &hash1).await.unwrap();

        let raw2 = mint_token();
        let hash2 = hash_token(&raw2);
        store.put_token("myapp", "tid-1", &hash2).await.unwrap();

        // Old hash no longer authenticates.
        assert!(store.app_for_token(&hash1).await.unwrap().is_none());
        assert_eq!(
            store.app_for_token(&hash2).await.unwrap(),
            Some(("myapp".to_string(), "tid-1".to_string()))
        );

        // Revoking the token id kills the current hash too.
        store.remove_token("myapp", "tid-1").await.unwrap();
        assert!(store.app_for_token(&hash2).await.unwrap().is_none());
        assert!(store.app_for_token(&hash1).await.unwrap().is_none());
    }

    #[test]
    fn minted_tokens_are_unique_and_prefixed() {
        let a = mint_token();
        let b = mint_token();
        assert_ne!(a, b);
        assert!(a.starts_with("mrl_live_"));
        assert!(a.len() > "mrl_live_".len() + 30);
    }

    #[test]
    fn hash_is_sha256_hex_of_raw_token() {
        let raw = mint_token();
        let hash = hash_token(&raw);
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn rpm_counter_increments_and_resets_per_window() {
        let store = MemoryStore::new();
        assert_eq!(store.incr_rpm("myapp", 1000).await.unwrap(), 1);
        assert_eq!(store.incr_rpm("myapp", 1000).await.unwrap(), 2);
        // A different minute window starts over.
        assert_eq!(store.incr_rpm("myapp", 1001).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn spend_accumulates_per_day() {
        let store = MemoryStore::new();
        assert_eq!(
            store.add_spend("myapp", "2026-11-05", 1.5).await.unwrap(),
            1.5
        );
        assert_eq!(
            store.add_spend("myapp", "2026-11-05", 2.25).await.unwrap(),
            3.75
        );
        assert_eq!(store.spend("myapp", "2026-11-05").await.unwrap(), 3.75);
        // A different day is a separate bucket.
        assert_eq!(store.spend("myapp", "2026-11-06").await.unwrap(), 0.0);
    }

    /// Integration test: only runs when MERLIN_TEST_REDIS_URL is set.
    /// Start a local Redis to run it:
    ///   docker run --rm -p 6379:6379 redis:7
    ///   MERLIN_TEST_REDIS_URL=redis://127.0.0.1:6379 cargo test --lib governance::apps::tests::redis_store_roundtrip
    #[tokio::test]
    async fn redis_store_roundtrip() {
        let url = match std::env::var("MERLIN_TEST_REDIS_URL") {
            Ok(u) => u,
            Err(_) => {
                eprintln!("skipping: MERLIN_TEST_REDIS_URL not set");
                return;
            }
        };
        let store = RedisStore::connect(&url).await.unwrap();
        let suffix = uuid::Uuid::new_v4().to_string();
        let app_id = format!("testapp-{suffix}");

        let app = App::new(&app_id, "Redis Test");
        store.put_app(&app).await.unwrap();
        assert!(store
            .list_apps()
            .await
            .unwrap()
            .iter()
            .any(|a| a.id == app_id));
        assert_eq!(
            store.get_app(&app_id).await.unwrap().unwrap().name,
            "Redis Test"
        );

        let raw = mint_token();
        let hash = hash_token(&raw);
        store.put_token(&app_id, "tid-1", &hash).await.unwrap();
        assert_eq!(
            store.app_for_token(&hash).await.unwrap(),
            Some((app_id.clone(), "tid-1".to_string()))
        );

        assert_eq!(store.incr_rpm(&app_id, 1000).await.unwrap(), 1);
        assert_eq!(store.incr_rpm(&app_id, 1000).await.unwrap(), 2);
        assert!((store.add_spend(&app_id, "2026-11-05", 1.5).await.unwrap() - 1.5).abs() < 1e-9);
        assert!((store.spend(&app_id, "2026-11-05").await.unwrap() - 1.5).abs() < 1e-9);

        // Re-issue: the old hash must stop authenticating, the new one resolves.
        let raw2 = mint_token();
        let hash2 = hash_token(&raw2);
        store.put_token(&app_id, "tid-1", &hash2).await.unwrap();
        assert!(store.app_for_token(&hash).await.unwrap().is_none());
        assert_eq!(
            store.app_for_token(&hash2).await.unwrap(),
            Some((app_id.clone(), "tid-1".to_string()))
        );

        store.remove_token(&app_id, "tid-1").await.unwrap();
        assert!(store.app_for_token(&hash2).await.unwrap().is_none());

        // Cleanup so repeated runs against a shared Redis do not accumulate state.
        use redis::AsyncCommands;
        let mut conn = store.conn.clone();
        for key in [
            format!("gvn:app:{app_id}"),
            format!("gvn:app:{app_id}:tokens"),
            format!("gvn:app:{app_id}:rpm:1000"),
            format!("gvn:app:{app_id}:spend:2026-11-05"),
        ] {
            let _: redis::RedisResult<i64> = conn.del(key).await;
        }
        let _: redis::RedisResult<i64> = conn.srem("gvn:apps", &app_id).await;
    }
}
