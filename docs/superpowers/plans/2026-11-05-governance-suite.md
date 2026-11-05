# Governance Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a governance suite to merlin — per-app tokens, quotas/budgets, and a fail-closed policy gate that restricts bandit routing to an eligible target set — per the approved spec at `docs/superpowers/specs/2026-11-05-governance-suite-design.md`.

**Architecture:** A new `src/governance/` module (detect / apps / gate) wired into `RouterEngine` as a pre-bandit filter. The server authenticates a bearer token, checks quotas, resolves sensitivity (sidecar → local detector → undetermined), computes eligibility, and calls `execute_governed`. The gate is fail-closed; routing stays fail-open. Governance is config-gated (`[governance] enabled = false` default) so existing deployments are untouched.

**Tech Stack:** Rust 2021, axum 0.7, tokio, redis 0.25 (already a dep, currently unused), regex + sha2 (new deps), proptest (new dev-dep for the property test), wiremock (existing dev-dep).

**Environment:** All cargo commands run from `/var/home/a/code/merlin` with `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib` prefixes (workspace `.cargo/config.toml` redirects target-dir to an unwritable path otherwise). Example:

```bash
cd /var/home/a/code/merlin && CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test
```

**Git:** Work on branch `feat/governance-suite`, created from the current HEAD. The working tree has pre-existing uncommitted changes (`Cargo.toml`, `merlin.toml`, `src/clients/translating.rs`, untracked `src/routing/bandit_verified.rs`) — never `git add -A`; every commit step lists exact paths.

---

## File Structure

| File | Responsibility |
|---|---|
| `src/governance/mod.rs` | Module root; re-exports detect/apps/gate |
| `src/governance/detect.rs` | Local sensitivity detector (regex + entropy), three-state verdict |
| `src/governance/apps.rs` | `App`, `AppPolicy`, `Store` trait, `MemoryStore`, `RedisStore`, token minting/hashing |
| `src/governance/gate.rs` | Pure eligibility function + `host_is_local` helper |
| `src/config.rs` | `GovernanceConfig`, `TargetConfig` price fields |
| `src/engine.rs` | `route_scope`, `target_locality`, `target_price`, `execute_governed`, `decide_governed`, `prompt_features` accessor |
| `src/metrics/mod.rs` | `merlin_governance_denials_total{code}` counter |
| `src/server.rs` | Auth/quota/gate pipeline, `ServerState.governance`, trace attachment |
| `src/main.rs` | `merlin app *` CLI subcommands |
| `tests/governance_test.rs` | Server-level governance integration tests |
| `examples/governance.toml` | Sample config |
| `README.md` | Governance section |

---

### Task 0: Branch and dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Create the feature branch**

```bash
cd /var/home/a/code/merlin && git checkout -b feat/governance-suite
```

Expected: `Switched to a new branch 'feat/governance-suite'`

- [ ] **Step 2: Add dependencies**

In `Cargo.toml`, add to `[dependencies]`:

```toml
regex = "1"
sha2 = "0.10"
```

and to `[dev-dependencies]`:

```toml
proptest = "1"
```

- [ ] **Step 3: Verify the crate still compiles**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo check`
Expected: finishes with `warning:` lines only, no errors.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock && git commit -m "build: add regex, sha2, proptest for governance suite"
```

---

### Task 1: Config — governance section, target pricing, locality helper

**Files:**
- Modify: `src/config.rs` (add `GovernanceConfig` after `FeaturesConfig`; add price fields to `TargetConfig`)
- Test: unit tests at the bottom of `src/config.rs`

- [ ] **Step 1: Write the failing test**

Append to the end of `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governance_config_defaults_when_absent() {
        let config: MerlinConfig = toml::from_str(
            "[clients]\n[targets]\n[routes]\n",
        )
        .expect("parse empty config");
        assert!(!config.governance.enabled);
        assert_eq!(config.governance.sensitivity_threshold, 0.7);
        assert!(config.governance.redis_url.is_none());
    }

    #[test]
    fn governance_config_parses_overrides() {
        let config: MerlinConfig = toml::from_str(
            "[governance]\nenabled = true\nsensitivity_threshold = 0.5\nredis_url = \"redis://10.0.0.1:6379\"\n",
        )
        .expect("parse governance config");
        assert!(config.governance.enabled);
        assert_eq!(config.governance.sensitivity_threshold, 0.5);
        assert_eq!(
            config.governance.redis_url.as_deref(),
            Some("redis://10.0.0.1:6379")
        );
    }

    #[test]
    fn target_pricing_defaults_to_none() {
        let target: TargetConfig =
            toml::from_str("client = \"openai\"\nmodel = \"gpt-4\"\n").unwrap();
        assert!(target.price_input.is_none());
        assert!(target.price_output.is_none());
    }

    #[test]
    fn target_pricing_parses() {
        let target: TargetConfig = toml::from_str(
            "client = \"openai\"\nmodel = \"gpt-4\"\nprice_input = 30.0\nprice_output = 60.0\n",
        )
        .unwrap();
        assert_eq!(target.price_input, Some(30.0));
        assert_eq!(target.price_output, Some(60.0));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib config::tests`
Expected: FAIL — `no field governance on MerlinConfig`, `no field price_input on TargetConfig`.

- [ ] **Step 3: Implement**

In `src/config.rs`, add the governance field to `MerlinConfig` (after `features`):

```rust
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MerlinConfig {
    pub server: ServerConfig,
    pub clients: HashMap<String, ClientConfig>,
    pub targets: HashMap<String, TargetConfig>,
    pub routes: HashMap<String, RouteConfig>,
    #[serde(default)]
    pub features: FeaturesConfig,
    #[serde(default)]
    pub governance: GovernanceConfig,
}
```

Add the struct after the `FeaturesConfig` `impl Default` block:

```rust
/// Governance (policy gate, app tokens, quotas) configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct GovernanceConfig {
    pub enabled: bool,
    /// Redis URL for the app/token/usage store. Falls back to
    /// `MERLIN_REDIS_URL`, then `redis://127.0.0.1:6379`.
    pub redis_url: Option<String>,
    /// Sidecar `is_sensitive` probability at or above which a request
    /// counts as sensitive.
    pub sensitivity_threshold: f64,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            redis_url: None,
            sensitivity_threshold: 0.7,
        }
    }
}
```

Add price fields to `TargetConfig` (after `extra_body`):

> **Implementation note (applied during execution):** `#[serde(default)]` on
> `MerlinConfig` is kept only on `server`, `features`, and `governance` —
> NOT on `clients`/`targets`/`routes` — so a typo'd `[governance]` section
> fails loudly at startup instead of silently leaving the gate disabled.
> `MerlinConfig::validate()` additionally rejects a non-finite or
> out-of-range (not in 0.0..=1.0) `sensitivity_threshold`.

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_body: Option<crate::protocol::tools::ExtraBody>,
    /// USD per 1M input tokens; backs daily-spend accounting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_input: Option<f64>,
    /// USD per 1M output tokens; backs daily-spend accounting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_output: Option<f64>,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib config::tests`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs && git commit -m "feat(config): governance section and per-target pricing"
```

---

### Task 2: Local sensitivity detector

**Files:**
- Create: `src/governance/mod.rs`
- Create: `src/governance/detect.rs`
- Modify: `src/lib.rs` (add `pub mod governance;`)

- [ ] **Step 1: Write the failing test**

Create `src/governance/detect.rs` with tests first:

```rust
//! Local sensitivity detector: deterministic fallback when the laya
//! sidecar is unavailable. Three-state verdict — `Sensitive`, `Clean`,
//! `Undetermined` (only when the scan itself could not run, i.e. the
//! prompt exceeds the scan size cap).

use regex::Regex;

/// Result of a local sensitivity scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanVerdict {
    Sensitive,
    Clean,
    Undetermined,
}

/// Prompts longer than this are not scanned and reported as
/// `Undetermined` (the gate then applies the strictest policy).
const MAX_SCAN_CHARS: usize = 1_000_000;

/// Minimum Shannon entropy (bits per char) for a whitespace-delimited
/// token to count as a probable secret.
const ENTROPY_THRESHOLD: f64 = 4.0;

/// Minimum length of a token considered for entropy screening.
const ENTROPY_MIN_LEN: usize = 20;

/// Clone so server state can share one detector across request handlers.
/// `Regex` is cheap to clone (compiles once, shares the program).
#[derive(Clone)]
pub struct Detector {
    patterns: Vec<Regex>,
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector {
    pub fn new() -> Self {
        let patterns = [
            // US SSN
            r"\b\d{3}-\d{2}-\d{4}\b",
            // Email address
            r"\b[\w.+-]+@[\w-]+\.[\w.-]+\b",
            // US-style phone
            r"\b(?:\+?1[-. ]?)?\(?\d{3}\)?[-. ]?\d{3}[-. ]?\d{4}\b",
            // AWS access key id
            r"\b(?:AKIA|ASIA|ABIA|ACCA)[A-Z0-9]{16}\b",
            // GitHub / GitLab tokens
            r"\b(?:ghp|gho|ghu|ghs|ghr|glpat)_[A-Za-z0-9]{20,}\b",
        ]
        .iter()
        .map(|p| Regex::new(p).expect("detector pattern compiles"))
        .collect();
        Self { patterns }
    }

    pub fn scan(&self, text: &str) -> ScanVerdict {
        if text.chars().count() > MAX_SCAN_CHARS {
            return ScanVerdict::Undetermined;
        }
        if self.patterns.iter().any(|re| re.is_match(text)) {
            return ScanVerdict::Sensitive;
        }
        if has_high_entropy_token(text) {
            return ScanVerdict::Sensitive;
        }
        ScanVerdict::Clean
    }
}

/// Shannon entropy (bits per character) of a token over its byte alphabet.
fn shannon_entropy(token: &str) -> f64 {
    let mut counts = [0usize; 256];
    let bytes = token.as_bytes();
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let len = bytes.len() as f64;
    counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
}

fn has_high_entropy_token(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        token.len() >= ENTROPY_MIN_LEN && shannon_entropy(token) >= ENTROPY_THRESHOLD
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssn_is_sensitive() {
        assert_eq!(
            Detector::new().scan("my ssn is 123-45-6789, thanks"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn email_is_sensitive() {
        assert_eq!(
            Detector::new().scan("contact me at jane.doe@example.com please"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn phone_is_sensitive() {
        assert_eq!(
            Detector::new().scan("call me at (415) 555-2671"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn aws_key_is_sensitive() {
        assert_eq!(
            Detector::new().scan("key: AKIAIOSFODNN7EXAMPLE and done"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn github_token_is_sensitive() {
        assert_eq!(
            Detector::new().scan("token ghp_16C7e42F292c6912E7710c838347Ae178B4a"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn high_entropy_token_is_sensitive() {
        // 40-char random-ish secret with no spaces.
        let secret = "x9Kq2mP8vL4zR7tN3wB6yH5sD1fG0jC8aE3uI";
        assert_eq!(Detector::new().scan(secret), ScanVerdict::Sensitive);
    }

    #[test]
    fn ordinary_prompt_is_clean() {
        assert_eq!(
            Detector::new().scan("What is the capital of France? Explain briefly."),
            ScanVerdict::Clean
        );
    }

    #[test]
    fn empty_prompt_is_clean() {
        assert_eq!(Detector::new().scan(""), ScanVerdict::Clean);
    }

    #[test]
    fn oversized_prompt_is_undetermined() {
        let big = "a".repeat(MAX_SCAN_CHARS + 1);
        assert_eq!(Detector::new().scan(&big), ScanVerdict::Undetermined);
    }
}
```

Create `src/governance/mod.rs`:

```rust
pub mod apps;
pub mod detect;
pub mod gate;
```

- [ ] **Step 2: Register the module and verify the test fails to build**

Add `pub mod governance;` to `src/lib.rs` (alphabetical order, after `features`).

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib governance::detect`
Expected: compile ERROR — `file not found for module apps` / `gate` (they don't exist yet).

- [ ] **Step 3: Create empty stubs so the module compiles**

Create `src/governance/apps.rs`:

```rust
//! App identity, policy, and token storage. (Filled in Task 3.)
```

Create `src/governance/gate.rs`:

```rust
//! Eligibility gate. (Filled in Task 5.)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib governance::detect`
Expected: 9 passed.

- [ ] **Step 5: Commit**

```bash
git add src/governance/mod.rs src/governance/detect.rs src/governance/apps.rs src/governance/gate.rs src/lib.rs && git commit -m "feat(governance): local sensitivity detector with entropy screening"
```

---

### Task 3: App types, token minting, and the Store trait with MemoryStore

**Files:**
- Modify: `src/governance/apps.rs`

- [ ] **Step 1: Write the failing test**

Replace the stub `src/governance/apps.rs` with tests first (implementation types referenced below):

```rust
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
        assert_eq!(store.add_spend("myapp", "2026-11-05", 1.5).await.unwrap(), 1.5);
        assert_eq!(store.add_spend("myapp", "2026-11-05", 2.25).await.unwrap(), 3.75);
        assert_eq!(store.spend("myapp", "2026-11-05").await.unwrap(), 3.75);
        // A different day is a separate bucket.
        assert_eq!(store.spend("myapp", "2026-11-06").await.unwrap(), 0.0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib governance::apps`
Expected: FAIL — `MemoryStore`, `App`, `mint_token`, `hash_token` unresolved.

- [ ] **Step 3: Implement**

Replace `src/governance/apps.rs` with:

```rust
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
    /// Daily spend cap in USD; `None` = unlimited.
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
#[async_trait]
pub trait Store: Send + Sync {
    async fn put_app(&self, app: &App) -> Result<(), StoreError>;
    async fn get_app(&self, id: &str) -> Result<Option<App>, StoreError>;
    async fn list_apps(&self) -> Result<Vec<App>, StoreError>;

    /// Record a token hash under `token_id` for `app_id`.
    async fn put_token(&self, app_id: &str, token_id: &str, hash: &str)
        -> Result<(), StoreError>;
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
        self.inner.write().await.apps.insert(app.id.clone(), app.clone());
        Ok(())
    }

    async fn get_app(&self, id: &str) -> Result<Option<App>, StoreError> {
        Ok(self.inner.read().await.apps.get(id).cloned())
    }

    async fn list_apps(&self) -> Result<Vec<App>, StoreError> {
        Ok(self.inner.read().await.apps.values().cloned().collect())
    }

    async fn put_token(
        &self,
        app_id: &str,
        token_id: &str,
        hash: &str,
    ) -> Result<(), StoreError> {
        let mut inner = self.inner.write().await;
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
```

Keep the `#[cfg(test)] mod tests` from Step 1 at the bottom of the file.

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib governance::apps`
Expected: 7 passed.

- [ ] **Step 5: Commit**

```bash
git add src/governance/apps.rs && git commit -m "feat(governance): app policy types, token minting, Store trait with MemoryStore"
```

---

### Task 4: RedisStore

**Files:**
- Modify: `src/governance/apps.rs` (append `RedisStore`)

- [ ] **Step 1: Write the (env-gated) test**

Append to the `#[cfg(test)] mod tests` in `src/governance/apps.rs`:

```rust
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
        assert_eq!(store.get_app(&app_id).await.unwrap().unwrap().name, "Redis Test");

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

        store.remove_token(&app_id, "tid-1").await.unwrap();
        assert!(store.app_for_token(&hash).await.unwrap().is_none());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib governance::apps`
Expected: FAIL — `RedisStore` unresolved.

- [ ] **Step 3: Implement**

Append to `src/governance/apps.rs`:

```rust
/// Redis-backed store. Keys:
///
/// ```text
/// gvn:app:<id>                 hash    {json: <App JSON>}
/// gvn:app:<id>:tokens          hash    {token_id: sha256}
/// gvn:tok:<sha256>             hash    {app: <id>, tid: <token_id>}
/// gvn:app:<id>:rpm:<minute>    string  INCR, EXPIRE 60
/// gvn:app:<id>:spend:<day>     string  INCRBYFLOAT, EXPIRE 172800
/// ```
pub struct RedisStore {
    conn: redis::aio::ConnectionManager,
}

impl RedisStore {
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let client = redis::Client::open(url)
            .map_err(|e| StoreError::Backend(format!("bad redis url: {e}")))?;
        let conn = redis::aio::ConnectionManager::new(client)
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
        self.conn
            .clone()
            .hset::<_, _, _, ()>(format!("gvn:app:{}", app.id), "json", json)
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
        // Redis has no cheap hash-of-hashes scan; list via token-free
        // app keys. Operators list apps with the CLI, which keys off
        // a dedicated index set.
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

    async fn put_token(
        &self,
        app_id: &str,
        token_id: &str,
        hash: &str,
    ) -> Result<(), StoreError> {
        use redis::AsyncCommands;
        let mut conn = self.conn.clone();
        conn.sadd::<_, _, ()>("gvn:apps", app_id)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        conn.hset::<_, _, _, ()>(format!("gvn:app:{app_id}:tokens"), token_id, hash)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        conn.hset_multiple::<_, _, ()>(
            format!("gvn:tok:{hash}"),
            &[("app", app_id), ("tid", token_id)],
        )
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))
    }

    async fn remove_token(&self, app_id: &str, token_id: &str) -> Result<(), StoreError> {
        use redis::AsyncCommands;
        let hash: Option<String> = self
            .conn
            .clone()
            .hget(format!("gvn:app:{app_id}:tokens"), token_id)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let mut conn = self.conn.clone();
        conn.hdel::<_, _, ()>(format!("gvn:app:{app_id}:tokens"), token_id)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        if let Some(hash) = hash {
            conn.del::<_, ()>(format!("gvn:tok:{hash}"))
                .await
                .map_err(|e| StoreError::Backend(e.to_string()))?;
        }
        Ok(())
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
            let _: redis::RedisResult<i64> = self.conn.clone().expire(&key, 90).await;
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
            Some(v) => v.parse::<f64>().map_err(|e| StoreError::Backend(e.to_string())),
            None => Ok(0.0),
        }
    }

    async fn add_spend(&self, app_id: &str, day: &str, amount: f64) -> Result<f64, StoreError> {
        use redis::AsyncCommands;
        let key = format!("gvn:app:{app_id}:spend:{day}");
        let mut conn = self.conn.clone();
        let total: f64 = conn
            .incr_by_float(&key, amount)
            .await
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        // Best-effort TTL: only set when the key is fresh (count 1).
        let exists: i64 = conn.exists(&key).await.unwrap_or(1);
        if exists == 1 {
            let _: redis::RedisResult<i64> = conn.expire(&key, 172_800).await;
        }
        Ok(total)
    }
}
```

- [ ] **Step 4: Run tests (skipped path) and compile check**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib governance::apps`
Expected: 7 passed, 1 skipped (prints "skipping: MERLIN_TEST_REDIS_URL not set"). If a local Redis is available: `docker run --rm -d -p 6379:6379 redis:7`, then run with `MERLIN_TEST_REDIS_URL=redis://127.0.0.1:6379` → 8 passed.

- [ ] **Step 5: Commit**

```bash
git add src/governance/apps.rs && git commit -m "feat(governance): Redis-backed store for apps, tokens, and usage"
```

---

### Task 5: Eligibility gate (pure function)

**Files:**
- Modify: `src/governance/gate.rs`

- [ ] **Step 1: Write the failing test**

Replace the stub `src/governance/gate.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn scope(names: &[(&str, bool)]) -> Vec<TargetScope> {
        names
            .iter()
            .map(|(n, local)| TargetScope {
                name: n.to_string(),
                local: *local,
            })
            .collect()
    }

    #[test]
    fn default_policy_allows_everything() {
        let policy = AppPolicy::default();
        let result = evaluate(&policy, Sensitivity::Clean, &scope(&[("a", false), ("b", true)]));
        assert_eq!(result.unwrap(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn local_only_keeps_local_targets() {
        let policy = AppPolicy {
            locality: LocalityPolicy::LocalOnly,
            ..Default::default()
        };
        let result = evaluate(&policy, Sensitivity::Clean, &scope(&[("cloud", false), ("ollama", true)]));
        assert_eq!(result.unwrap(), vec!["ollama".to_string()]);
    }

    #[test]
    fn local_only_with_no_local_targets_denies_with_conflict() {
        let policy = AppPolicy {
            locality: LocalityPolicy::LocalOnly,
            ..Default::default()
        };
        let denial = evaluate(&policy, Sensitivity::Clean, &scope(&[("cloud", false)])).unwrap_err();
        assert_eq!(denial.code, "policy_denied");
        assert!(denial.conflicts.iter().any(|c| c.contains("no local targets")));
    }

    #[test]
    fn allowed_targets_allowlist_filters() {
        let policy = AppPolicy {
            allowed_targets: Some(vec!["b".to_string()]),
            ..Default::default()
        };
        let result = evaluate(&policy, Sensitivity::Clean, &scope(&[("a", false), ("b", true)]));
        assert_eq!(result.unwrap(), vec!["b".to_string()]);
    }

    #[test]
    fn sensitive_with_raise_only_restricts_to_local() {
        let policy = AppPolicy {
            sensitive_local_only: true,
            ..Default::default()
        };
        let sensitive = evaluate(&policy, Sensitivity::Sensitive, &scope(&[("cloud", false), ("ollama", true)]));
        assert_eq!(sensitive.unwrap(), vec!["ollama".to_string()]);
        // Clean requests are unaffected.
        let clean = evaluate(&policy, Sensitivity::Clean, &scope(&[("cloud", false), ("ollama", true)]));
        assert_eq!(clean.unwrap().len(), 2);
    }

    #[test]
    fn sensitive_without_raise_only_stays_on_default_policy() {
        let policy = AppPolicy::default();
        let result = evaluate(&policy, Sensitivity::Sensitive, &scope(&[("cloud", false), ("ollama", true)]));
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn undetermined_is_strict_even_without_raise_only() {
        let policy = AppPolicy::default();
        let result = evaluate(&policy, Sensitivity::Undetermined, &scope(&[("cloud", false), ("ollama", true)]));
        assert_eq!(result.unwrap(), vec!["ollama".to_string()]);
    }

    #[test]
    fn undetermined_with_no_local_targets_denies() {
        let policy = AppPolicy::default();
        let denial = evaluate(&policy, Sensitivity::Undetermined, &scope(&[("cloud", false)])).unwrap_err();
        assert!(denial.conflicts.iter().any(|c| c.contains("undetermined")));
    }

    #[test]
    fn allowlist_and_locality_compose() {
        let policy = AppPolicy {
            allowed_targets: Some(vec!["cloud".to_string()]),
            locality: LocalityPolicy::LocalOnly,
            ..Default::default()
        };
        let denial = evaluate(&policy, Sensitivity::Clean, &scope(&[("cloud", false), ("ollama", true)]));
        assert!(denial.is_err());
        let both = evaluate(
            &policy,
            Sensitivity::Clean,
            &scope(&[("cloud", false), ("local-b", true)]),
        );
        assert_eq!(both.unwrap(), vec!["local-b".to_string()]);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib governance::gate`
Expected: FAIL — `TargetScope`, `Sensitivity`, `evaluate`, `AppPolicy`, `LocalityPolicy` unresolved.

- [ ] **Step 3: Implement**

Replace `src/governance/gate.rs` with:

```rust
//! Eligibility gate: pure function from (app policy, sensitivity, route
//! scope) to the eligible target set. An empty set is a denial with
//! self-explaining conflicts — never a fall-through to a cheaper target.

use super::apps::{AppPolicy, LocalityPolicy};

/// Resolved sensitivity of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sensitivity {
    Clean,
    Sensitive,
    /// Sensitivity could not be determined (detector unavailable); the
    /// gate fails closed and applies the strictest policy.
    Undetermined,
}

/// One candidate target: its name and whether it runs locally.
#[derive(Debug, Clone)]
pub struct TargetScope {
    pub name: String,
    pub local: bool,
}

/// Why a request was denied, in terms the caller can act on. Hints never
/// suggest relaxing a security requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Denial {
    pub code: &'static str,
    pub conflicts: Vec<String>,
}

/// Compute the eligible target set for a request.
///
/// Restrictions compose:
/// - `allowed_targets` allowlist (if set) intersects the scope;
/// - `locality = LocalOnly` keeps only local targets;
/// - `Sensitive` + `sensitive_local_only` keeps only local targets
///   (raise-only; never applied when the request is clean);
/// - `Undetermined` keeps only local targets regardless of policy
///   (fail-closed).
pub fn evaluate(
    policy: &AppPolicy,
    sensitivity: Sensitivity,
    scope: &[TargetScope],
) -> Result<Vec<String>, Denial> {
    let strict_local = sensitivity == Sensitivity::Undetermined;
    let restrict_local = policy.locality == LocalityPolicy::LocalOnly
        || (sensitivity == Sensitivity::Sensitive && policy.sensitive_local_only)
        || strict_local;

    let eligible: Vec<String> = scope
        .iter()
        .filter(|t| !restrict_local || t.local)
        .filter(|t| {
            policy
                .allowed_targets
                .as_ref()
                .map(|allowed| allowed.iter().any(|a| a == &t.name))
                .unwrap_or(true)
        })
        .map(|t| t.name.clone())
        .collect();

    if !eligible.is_empty() {
        return Ok(eligible);
    }

    let mut conflicts = Vec::new();
    let has_local = scope.iter().any(|t| t.local);
    if restrict_local {
        if strict_local {
            conflicts.push(
                "sensitivity undetermined: strict policy requires local targets, but none available"
                    .to_string(),
            );
        } else if has_local {
            conflicts.push("no local targets eligible under the active policy".to_string());
        } else {
            conflicts.push("policy requires local targets, but none are available".to_string());
        }
    }
    if let Some(allowed) = &policy.allowed_targets {
        if scope.iter().all(|t| !allowed.contains(&t.name)) {
            conflicts.push("allowed_targets excludes every target in this route".to_string());
        }
    }
    if conflicts.is_empty() {
        conflicts.push("no targets in scope".to_string());
    }

    Err(Denial {
        code: "policy_denied",
        conflicts,
    })
}

/// True when `base_url`'s host is localhost, loopback, or a private-range
/// IP — i.e. traffic to this client stays on local infrastructure.
pub fn host_is_local(base_url: &str) -> bool {
    let host = base_url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split(['/', ':'])
        .next()
        .unwrap_or("")
        .trim_matches(['[', ']']);
    host == "localhost"
        || host.starts_with("127.")
        || host == "::1"
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
        || host
            .strip_prefix("172.")
            .and_then(|rest| rest.split('.').next()?.parse::<u8>().ok())
            .map(|second| (16..=31).contains(&second))
            .unwrap_or(false)
}
```

Keep the tests from Step 1 at the bottom, and add locality tests to the same module:

```rust
    #[test]
    fn locality_derivation() {
        assert!(host_is_local("http://127.0.0.1:11434"));
        assert!(host_is_local("http://localhost:8000/v1"));
        assert!(host_is_local("http://10.0.0.5:9000"));
        assert!(host_is_local("http://192.168.1.20"));
        assert!(host_is_local("http://172.16.0.5"));
        assert!(host_is_local("http://172.31.255.1"));
        assert!(host_is_local("http://[::1]:8080"));
        assert!(!host_is_local("https://api.openai.com/v1"));
        assert!(!host_is_local("https://172.32.0.1"));
        assert!(!host_is_local("http://172.15.0.1"));
        assert!(!host_is_local("https://example.com"));
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --lib governance::gate`
Expected: 10 passed.

- [ ] **Step 5: Commit**

```bash
git add src/governance/gate.rs && git commit -m "feat(governance): eligibility gate with fail-closed undetermined handling"
```

---

### Task 6: Property test — eligible set never contains a forbidden target

**Files:**
- Create: `tests/gate_property_test.rs`

- [ ] **Step 1: Write the property test**

Create `tests/gate_property_test.rs`:

```rust
//! Property test: for any policy, sensitivity, and route scope, the
//! eligible set never contains a target the policy forbids. This is the
//! fail-open canary — RouteForge's Hypothesis harness caught two such
//! defects; ours proves the invariant at the seam.

use merlin::governance::apps::{AppPolicy, LocalityPolicy};
use merlin::governance::gate::{evaluate, Sensitivity, TargetScope};
use proptest::prelude::*;

fn scope_strategy() -> impl Strategy<Value = Vec<TargetScope>> {
    proptest::collection::vec(
        ("[a-z]{1,8}", any::<bool>()),
        0..6,
    )
    .prop_map(|items| {
        items
            .into_iter()
            .map(|(name, local)| TargetScope { name, local })
            .collect()
    })
}

fn policy_strategy() -> impl Strategy<Value = AppPolicy> {
    (
        proptest::option::of(proptest::collection::vec("[a-z]{1,8}", 0..4)),
        any::<bool>(),
        proptest::option::of(0u32..1000),
        proptest::option::of(0.0f64..100.0),
        any::<bool>(),
    )
        .prop_map(
            |(allowed_targets, local_only, max_rpm, max_spend, sensitive_local_only)| {
                AppPolicy {
                    allowed_targets,
                    locality: if local_only {
                        LocalityPolicy::LocalOnly
                    } else {
                        LocalityPolicy::Any
                    },
                    max_rpm,
                    max_spend_usd_per_day: max_spend,
                    sensitive_local_only,
                    ..Default::default()
                }
            },
        )
}

proptest! {
    #[test]
    fn eligible_set_never_contains_forbidden_target(
        policy in policy_strategy(),
        sensitive in any::<bool>(),
        undetermined in any::<bool>(),
        scope in scope_strategy(),
    ) {
        let sensitivity = if undetermined {
            Sensitivity::Undetermined
        } else if sensitive {
            Sensitivity::Sensitive
        } else {
            Sensitivity::Clean
        };

        if let Ok(eligible) = evaluate(&policy, sensitivity, &scope) {
            let restricted = policy.locality == LocalityPolicy::LocalOnly
                || (sensitivity == Sensitivity::Sensitive && policy.sensitive_local_only)
                || sensitivity == Sensitivity::Undetermined;
            for name in &eligible {
                let target = scope.iter().find(|t| &t.name == name).expect("name from scope");
                prop_assert!(!restricted || target.local,
                    "non-local target {name} in eligible set under local restriction");
                if let Some(allowed) = &policy.allowed_targets {
                    prop_assert!(allowed.contains(name),
                        "target {name} not in allowed_targets");
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run the property test**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --test gate_property_test`
Expected: PASS (256 cases).

- [ ] **Step 3: Commit**

```bash
git add tests/gate_property_test.rs && git commit -m "test(governance): property test that eligibility never violates policy"
```

---

### Task 7: Engine — governed execution over an eligible set

**Files:**
- Modify: `src/engine.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/governed_engine_test.rs`:

```rust
//! Governed engine path: eligibility filtering before bandit selection.

use merlin::config::{AlgorithmConfig, ClientConfig, MerlinConfig, RouteConfig, TargetConfig};
use merlin::engine::RouterEngine;
use merlin::protocol::LlmResponse;

fn test_config(local_url: &str, cloud_url: &str) -> MerlinConfig {
    let mut config = MerlinConfig::default();
    config.server.port = 0;
    config.clients.insert(
        "local".to_string(),
        ClientConfig {
            format: "openai_chat".to_string(),
            base_url: local_url.to_string(),
            api_key_env: None,
            max_retries: 0,
            forward_auth: false,
            extra_headers: Default::default(),
            timeout_ms: 5000,
        },
    );
    config.clients.insert(
        "cloud".to_string(),
        ClientConfig {
            format: "openai_chat".to_string(),
            base_url: cloud_url.to_string(),
            api_key_env: None,
            max_retries: 0,
            forward_auth: false,
            extra_headers: Default::default(),
            timeout_ms: 5000,
        },
    );
    config.targets.insert(
        "local_t".to_string(),
        TargetConfig {
            name: "local_t".to_string(),
            client: "local".to_string(),
            model: "local-model".to_string(),
            extra_body: None,
            price_input: None,
            price_output: None,
        },
    );
    config.targets.insert(
        "cloud_t".to_string(),
        TargetConfig {
            name: "cloud_t".to_string(),
            client: "cloud".to_string(),
            model: "cloud-model".to_string(),
            extra_body: None,
            price_input: Some(10.0),
            price_output: Some(20.0),
        },
    );
    config.routes.insert(
        "default".to_string(),
        RouteConfig {
            id: "default".to_string(),
            default_model: None,
            targets: vec!["local_t".to_string(), "cloud_t".to_string()],
            algorithm: AlgorithmConfig::EpsilonGreedy {
                targets: vec!["local_t".to_string(), "cloud_t".to_string()],
                epsilon: 0.0,
            },
        },
    );
    config
}

#[tokio::test]
async fn governed_execute_stays_inside_eligible_set() {
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    let local_mock = MockServer::start().await;
    let cloud_mock = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                          "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        })))
        .mount(&local_mock)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                          "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        })))
        .mount(&cloud_mock)
        .await;

    let engine = RouterEngine::new(test_config(&local_mock.uri(), &cloud_mock.uri())).unwrap();

    // Locality helpers.
    assert!(engine.target_locality("local_t"));
    assert!(!engine.target_locality("cloud_t"));
    assert_eq!(engine.target_price("cloud_t"), Some((10.0, 20.0)));
    assert_eq!(engine.target_price("local_t"), None);

    // Scope covers both route targets; eligibility filtering excludes cloud.
    let mut scope = engine.route_scope("default");
    scope.sort();
    assert_eq!(scope, vec!["cloud_t".to_string(), "local_t".to_string()]);

    let request = merlin::protocol::Request {
        llm_request: merlin::protocol::LlmRequest {
            model: Some("default".to_string()),
            messages: vec![merlin::protocol::Message::text(
                merlin::protocol::Role::User,
                "hello",
            )],
            ..Default::default()
        },
        ..Default::default()
    };

    let response = engine
        .execute_governed("default", request, Some(&["local_t".to_string()]))
        .await
        .expect("governed execute succeeds");
    match response {
        LlmResponse::Aggregate(agg) => {
            assert_eq!(
                agg.preservation.selected_target.as_deref(),
                Some("local_t")
            );
        }
        LlmResponse::Stream(_) => panic!("expected aggregate"),
    }
}

#[tokio::test]
async fn governed_execute_bails_when_algorithm_escapes_eligibility() {
    let mut config = test_config("http://127.0.0.1:1", "http://127.0.0.1:2");
    // Passthrough fixes its target at construction; governing it with an
    // eligible set that excludes it must fail, not silently violate policy.
    config.routes.insert(
        "pt".to_string(),
        RouteConfig {
            id: "pt".to_string(),
            default_model: None,
            targets: vec!["cloud_t".to_string()],
            algorithm: AlgorithmConfig::Passthrough {
                target: "cloud_t".to_string(),
            },
        },
    );
    let engine = RouterEngine::new(config).unwrap();
    let request = merlin::protocol::Request {
        llm_request: merlin::protocol::LlmRequest {
            model: Some("pt".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let err = engine
        .execute_governed("pt", request, Some(&["local_t".to_string()]))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("policy_denied"), "got: {err}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --test governed_engine_test`
Expected: compile ERROR — `target_locality`, `route_scope`, `target_price`, `execute_governed` unresolved.

- [ ] **Step 3: Implement**

In `src/engine.rs`, add these methods to `impl RouterEngine` (after `decide`):

```rust
    /// All target names reachable by a route: its declared targets plus
    /// any targets the algorithm fixes internally (passthrough,
    /// classifier). The gate scopes eligibility over exactly this set.
    pub fn route_scope(&self, route_name: &str) -> Vec<String> {
        let route = match self.routes.get(route_name) {
            Some(route) => route,
            None => return Vec::new(),
        };
        let mut names = route.config.targets.clone();
        names.extend(route.config.algorithm.target_names());
        names.sort();
        names.dedup();
        names
    }

    /// True when the target's client runs on local infrastructure
    /// (loopback/private-range base_url).
    pub fn target_locality(&self, target_name: &str) -> bool {
        self.config
            .targets
            .get(target_name)
            .and_then(|t| self.config.clients.get(&t.client))
            .map(|c| crate::governance::gate::host_is_local(&c.base_url))
            .unwrap_or(false)
    }

    /// (price_input, price_output) in USD per 1M tokens, if priced.
    pub fn target_price(&self, target_name: &str) -> Option<(f64, f64)> {
        let target = self.config.targets.get(target_name)?;
        Some((target.price_input?, target.price_output?))
    }

    /// Like [`RouterEngine::execute`], but restricted to `eligible` target
    /// names (from the governance gate). `None` = no restriction (plain
    /// execute). Bails with `policy_denied` when the route's algorithm
    /// fixes targets outside the eligible set (passthrough, classifier) —
    /// those algorithms cannot be arm-filtered at select time.
    pub async fn execute_governed(
        &self,
        route_name: &str,
        request: Request,
        eligible: Option<&[String]>,
    ) -> anyhow::Result<LlmResponse> {
        if let Some(eligible) = eligible {
            self.check_algorithm_scope(route_name, eligible)?;
        }
        self.execute_inner(route_name, request, eligible).await
    }

    /// Governed variant of [`RouterEngine::decide`].
    pub async fn decide_governed(
        &self,
        route_name: &str,
        request: Request,
        eligible: Option<&[String]>,
    ) -> anyhow::Result<RoutingOutcome> {
        if let Some(eligible) = eligible {
            self.check_algorithm_scope(route_name, eligible)?;
        }
        self.decide_inner(route_name, request, eligible).await
    }

    fn check_algorithm_scope(
        &self,
        route_name: &str,
        eligible: &[String],
    ) -> anyhow::Result<()> {
        let route = self
            .routes
            .get(route_name)
            .ok_or_else(|| anyhow::anyhow!("unknown route: {}", route_name))?;
        let fixed = route.config.algorithm.target_names();
        let escapes: Vec<String> = fixed
            .iter()
            .filter(|name| !eligible.iter().any(|e| e == *name))
            .cloned()
            .collect();
        if !escapes.is_empty() {
            anyhow::bail!(
                "policy_denied: algorithm-fixed targets outside eligible set: {}",
                escapes.join(", ")
            );
        }
        Ok(())
    }
```

Refactor `execute` and `decide` into inner functions taking `eligible: Option<&[String]>`:

> **Implementation note (applied during execution):** the plan originally
> assumed algorithms honor the `targets` argument to `select()`. In fact
> ALL arm-based algorithms (epsilon_greedy, thompson, ucb, random,
> contextual) select from their construction-time arm lists and ignore
> the argument (verified in src/routing/*). Governance therefore
> enforces eligibility fail-closed AFTER selection in a single shared
> `select_governed` helper: candidates are pre-filtered (honored by
> passthrough/classifier), a selected target outside the eligible set
> bails `policy_denied` before any dispatch/metrics/reward side effects,
> and fallbacks are filtered to the eligible set. `check_algorithm_scope`
> from the original plan text was dropped as redundant. Tests pin the
> invariant incl. a zero-upstream-requests assertion.

Replace the body of `execute` (keep its signature) with:

```rust
    pub async fn execute(
        &self,
        route_name: &str,
        request: Request,
    ) -> anyhow::Result<LlmResponse> {
        self.execute_inner(route_name, request, None).await
    }

    async fn execute_inner(
        &self,
        route_name: &str,
        mut request: Request,
        eligible: Option<&[String]>,
    ) -> anyhow::Result<LlmResponse> {
        let route = self
            .routes
            .get(route_name)
            .ok_or_else(|| anyhow::anyhow!("unknown route: {}", route_name))?;

        if request.llm_request.model.is_none() {
            request.llm_request.model = route.config.default_model.clone();
        }

        let eligible_set: Option<std::collections::HashSet<&str>> =
            eligible.map(|list| list.iter().map(String::as_str).collect());

        let targets: Vec<TargetConfig> = route
            .config
            .targets
            .iter()
            .filter(|t| {
                eligible_set
                    .as_ref()
                    .map(|set| set.contains(t.as_str()))
                    .unwrap_or(true)
            })
            .filter_map(|t| self.config.targets.get(t).cloned())
            .collect();

        if targets.is_empty() {
            anyhow::bail!("policy_denied: no eligible targets in route {}", route_name);
        }

        self.attach_features(route, &mut request).await;

        let outcome = route.algorithm.select(&request, &targets)?;
        let selected = self.resolve_target(&outcome.selected)?;
        let fallbacks: Vec<TargetRef> = outcome
            .fallbacks
            .iter()
            .filter_map(|n| self.resolve_target(n).ok())
            .collect();

        let start = std::time::Instant::now();
        let result = self
            .dispatch_with_fallbacks(&selected, &fallbacks, request.clone(), route_name)
            .await;
        let duration = start.elapsed();

        match &result {
            Ok(resp) => {
                self.metrics
                    .record_request(route_name, &selected.name, true, duration);
                if let LlmResponse::Aggregate(agg) = resp {
                    self.metrics.record_tokens(
                        route_name,
                        &selected.name,
                        agg.usage.prompt_tokens + agg.usage.completion_tokens,
                    );
                }
                route.algorithm.record_reward(&selected.name, &request, 1.0);
            }
            Err(_) => {
                self.metrics
                    .record_request(route_name, &selected.name, false, duration);
                route.algorithm.record_reward(&selected.name, &request, 0.0);
            }
        }

        result
    }
```

(The original `execute` body is deleted; its contents now live in `execute_inner` with the eligibility filter added.)

Do the same for `decide`:

```rust
    pub async fn decide(
        &self,
        route_name: &str,
        request: Request,
    ) -> anyhow::Result<RoutingOutcome> {
        self.decide_inner(route_name, request, None).await
    }

    async fn decide_inner(
        &self,
        route_name: &str,
        mut request: Request,
        eligible: Option<&[String]>,
    ) -> anyhow::Result<RoutingOutcome> {
        let route = self
            .routes
            .get(route_name)
            .ok_or_else(|| anyhow::anyhow!("unknown route: {}", route_name))?;
        if request.llm_request.model.is_none() {
            request.llm_request.model = route.config.default_model.clone();
        }
        let eligible_set: Option<std::collections::HashSet<&str>> =
            eligible.map(|list| list.iter().map(String::as_str).collect());
        let targets: Vec<TargetConfig> = route
            .config
            .targets
            .iter()
            .filter(|t| {
                eligible_set
                    .as_ref()
                    .map(|set| set.contains(t.as_str()))
                    .unwrap_or(true)
            })
            .filter_map(|t| self.config.targets.get(t).cloned())
            .collect();
        if targets.is_empty() {
            anyhow::bail!("policy_denied: no eligible targets in route {}", route_name);
        }
        self.attach_features(route, &mut request).await;
        let outcome = route.algorithm.select(&request, &targets)?;
        Ok(RoutingOutcome {
            target: self.resolve_target(&outcome.selected)?,
            fallback_chain: outcome.fallbacks,
        })
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --test governed_engine_test`
Expected: 2 passed.

Also run the full lib suite for regressions: `cargo test --lib` — Expected: all passed.

- [ ] **Step 5: Commit**

```bash
git add src/engine.rs tests/governed_engine_test.rs && git commit -m "feat(engine): governed execution restricted to the eligible target set"
```

---

### Task 8: Metrics counter + server governance pipeline

**Files:**
- Modify: `src/metrics/mod.rs`
- Modify: `src/server.rs`
- Test: `tests/governance_test.rs`

- [ ] **Step 1: Write the failing integration test**

Create `tests/governance_test.rs`:

```rust
//! Server-level governance: auth, quotas, gate, trace.

use merlin::config::{AlgorithmConfig, ClientConfig, GovernanceConfig, MerlinConfig, RouteConfig, TargetConfig};
use merlin::engine::RouterEngine;
use merlin::governance::apps::{App, AppPolicy, MemoryStore, Store};
use merlin::governance::detect::Detector;
use merlin::governance::gate::Sensitivity;
use merlin::server::{create_app_with_governance, Governance};
use std::sync::Arc;

async fn governed_app(policy: AppPolicy) -> (axum::Router, Arc<MemoryStore>) {
    let local_mock = wiremock::MockServer::start().await;
    let cloud_mock = wiremock::MockServer::start().await;
    for mock in [&local_mock, &cloud_mock] {
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                              "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
            })))
            .mount(mock)
            .await;
    }

    let mut config = MerlinConfig::default();
    config.server.port = 0;
    config.clients.insert("local".to_string(), ClientConfig {
        format: "openai_chat".to_string(),
        base_url: local_mock.uri(),
        api_key_env: None, max_retries: 0, forward_auth: false,
        extra_headers: Default::default(), timeout_ms: 5000,
    });
    config.clients.insert("cloud".to_string(), ClientConfig {
        format: "openai_chat".to_string(),
        base_url: cloud_mock.uri(),
        api_key_env: None, max_retries: 0, forward_auth: false,
        extra_headers: Default::default(), timeout_ms: 5000,
    });
    config.targets.insert("local_t".to_string(), TargetConfig {
        name: "local_t".to_string(), client: "local".to_string(),
        model: "local-model".to_string(), extra_body: None,
        price_input: None, price_output: None,
    });
    config.targets.insert("cloud_t".to_string(), TargetConfig {
        name: "cloud_t".to_string(), client: "cloud".to_string(),
        model: "cloud-model".to_string(), extra_body: None,
        price_input: Some(10.0), price_output: Some(20.0),
    });
    config.routes.insert("default".to_string(), RouteConfig {
        id: "default".to_string(), default_model: None,
        targets: vec!["local_t".to_string(), "cloud_t".to_string()],
        algorithm: AlgorithmConfig::EpsilonGreedy {
            targets: vec!["local_t".to_string(), "cloud_t".to_string()],
            epsilon: 0.0,
        },
    });
    config.governance = GovernanceConfig {
        enabled: true,
        redis_url: None,
        sensitivity_threshold: 0.7,
    };

    let engine = RouterEngine::new(config).unwrap();
    let store = Arc::new(MemoryStore::new());
    let app = App::new("myapp", "My App");
    store.put_app(&app).await.unwrap();
    // IMPORTANT: the runtime Governance config must be enabled — tests
    // exercise the governed path.
    let gov_config = GovernanceConfig {
        enabled: true,
        redis_url: None,
        sensitivity_threshold: 0.7,
    };
    let gov = Governance {
        config: gov_config,
        store: store.clone(),
        detector: Detector::new(),
    };
    (create_app_with_governance(Arc::new(engine), gov), store)
}

fn chat_request() -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::json!({
                "model": "default",
                "messages": [{"role": "user", "content": "hello"}]
            })
            .to_string(),
        ))
        .unwrap()
}

#[tokio::test]
async fn missing_token_gets_401() {
    use tower::ServiceExt;
    let (app, _store) = governed_app(AppPolicy::default()).await;
    let response = app.oneshot(chat_request()).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valid_token_gets_trace_and_success() {
    use tower::ServiceExt;
    let (app, store) = governed_app(AppPolicy::default()).await;
    let raw = merlin::governance::apps::mint_token();
    let hash = merlin::governance::apps::hash_token(&raw);
    store.put_token("myapp", "tid-1", &hash).await.unwrap();

    let request = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {raw}"))
        .body(axum::body::Body::from(
            serde_json::json!({
                "model": "default",
                "messages": [{"role": "user", "content": "hello"}]
            })
            .to_string(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body["merlin_trace"].is_object(), "trace present: {body}");
    assert_eq!(body["merlin_trace"]["app"].as_str().unwrap(), "myapp");
}

#[tokio::test]
async fn over_rpm_gets_429() {
    use tower::ServiceExt;
    let (app, store) = governed_app(AppPolicy { max_rpm: Some(1), ..Default::default() }).await;
    let raw = merlin::governance::apps::mint_token();
    let hash = merlin::governance::apps::hash_token(&raw);
    store.put_token("myapp", "tid-1", &hash).await.unwrap();

    let req = || {
        axum::http::Request::builder()
            .method(axum::http::Method::POST)
            .uri("/v1/chat/completions")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {raw}"))
            .body(axum::body::Body::from(
                serde_json::json!({
                    "model": "default",
                    "messages": [{"role": "user", "content": "hello"}]
                })
                .to_string(),
            ))
            .unwrap()
    };
    let first = app.clone().oneshot(req()).await.unwrap();
    assert_eq!(first.status(), axum::http::StatusCode::OK);
    let second = app.oneshot(req()).await.unwrap();
    assert_eq!(second.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn sensitive_request_denied_when_no_local_targets() {
    use tower::ServiceExt;
    // Inline config (not the shared helper): the helper's wiremock
    // upstreams are 127.0.0.1, which host_is_local treats as LOCAL — so
    // a deny test needs a genuinely non-local client. "cloud" points at
    // example.com and is never contacted: the gate denies before any
    // upstream call.
    let mut config = MerlinConfig::default();
    config.server.port = 0;
    config.clients.insert("cloud".to_string(), ClientConfig {
        format: "openai_chat".to_string(),
        base_url: "https://example.com".to_string(),
        api_key_env: None, max_retries: 0, forward_auth: false,
        extra_headers: Default::default(), timeout_ms: 5000,
    });
    config.targets.insert("cloud_t".to_string(), TargetConfig {
        name: "cloud_t".to_string(), client: "cloud".to_string(),
        model: "cloud-model".to_string(), extra_body: None,
        price_input: None, price_output: None,
    });
    config.routes.insert("default".to_string(), RouteConfig {
        id: "default".to_string(), default_model: None,
        targets: vec!["cloud_t".to_string()],
        algorithm: AlgorithmConfig::Passthrough { target: "cloud_t".to_string() },
    });
    config.governance = GovernanceConfig {
        enabled: true,
        redis_url: None,
        sensitivity_threshold: 0.7,
    };

    let engine = RouterEngine::new(config).unwrap();
    let store = Arc::new(MemoryStore::new());
    let app = App::new("myapp", "My App");
    store.put_app(&app).await.unwrap();
    let raw = merlin::governance::apps::mint_token();
    let hash = merlin::governance::apps::hash_token(&raw);
    store.put_token("myapp", "tid-1", &hash).await.unwrap();

    let gov = Governance {
        config: config_governance_enabled(),
        store: store.clone(),
        detector: Detector::new(),
    };
    let app_router = create_app_with_governance(Arc::new(engine), gov);

    // Sidecar is disabled → local detector sees the SSN → Sensitive.
    // sensitive_local_only restricts to local targets; the route has only
    // cloud_t (non-local) → the eligible set is empty → 403.
    let request = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {raw}"))
        .body(axum::body::Body::from(
            serde_json::json!({
                "model": "default",
                "messages": [{"role": "user", "content": "my ssn is 123-45-6789"}]
            })
            .to_string(),
        ))
        .unwrap();
    let response = app_router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["error"]["code"], "policy_denied");
    assert!(body["error"]["conflicts"].is_array());
    assert!(!body["error"]["conflicts"].as_array().unwrap().is_empty());
}

fn config_governance_enabled() -> GovernanceConfig {
    GovernanceConfig {
        enabled: true,
        redis_url: None,
        sensitivity_threshold: 0.7,
    }
}

#[tokio::test]
async fn governance_disabled_allows_anonymous() {
    use tower::ServiceExt;
    // create_app (no governance) must behave exactly as before.
    let mut config = MerlinConfig::default();
    config.server.port = 0;
    let mock = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-test", "object": "chat.completion", "model": "m",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"},
                          "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        })))
        .mount(&mock)
        .await;
    config.clients.insert("c".to_string(), ClientConfig {
        format: "openai_chat".to_string(), base_url: mock.uri(), api_key_env: None,
        max_retries: 0, forward_auth: false, extra_headers: Default::default(), timeout_ms: 5000,
    });
    config.targets.insert("t".to_string(), TargetConfig {
        name: "t".to_string(), client: "c".to_string(), model: "m".to_string(),
        extra_body: None, price_input: None, price_output: None,
    });
    config.routes.insert("default".to_string(), RouteConfig {
        id: "default".to_string(), default_model: None,
        targets: vec!["t".to_string()],
        algorithm: AlgorithmConfig::Passthrough { target: "t".to_string() },
    });
    let engine = RouterEngine::new(config).unwrap();
    let response = merlin::server::create_app(Arc::new(engine))
        .oneshot(chat_request())
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    // Sensitivity enum is exercised here to keep the import used.
    assert_ne!(Sensitivity::Clean, Sensitivity::Sensitive);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --test governance_test`
Expected: compile ERROR — `create_app_with_governance`, `Governance` unresolved; metrics counter missing.

- [ ] **Step 3: Implement the metrics counter**

In `src/metrics/mod.rs`:

Add to the `Metrics` struct fields:

```rust
    governance_denials: prometheus::CounterVec,
```

In `Metrics::new`, before `Ok(Self {...})`:

```rust
        let governance_denials = prometheus::CounterVec::new(
            prometheus::opts!(
                "merlin_governance_denials_total",
                "Governance denials by error code"
            ),
            &["code"],
        )?;
        registry.register(Box::new(governance_denials.clone()))?;
```

Add to the `Ok(Self { ... })` initializer: `governance_denials,`.

Add method:

```rust
    pub fn record_governance_denial(&self, code: &str) {
        self.governance_denials.with_label_values(&[code]).inc();
    }
```

- [ ] **Step 4: Implement the server pipeline**

In `src/server.rs`, make these changes.

Add imports:

```rust
use crate::governance::apps::{hash_token, App, Store};
use crate::governance::detect::{Detector, ScanVerdict};
use crate::governance::gate::{self, Sensitivity, TargetScope};
```

Replace the `ServerState` struct and `create_app`:

```rust
#[derive(Clone)]
pub struct ServerState {
    engine: AppState,
    governance: Option<Governance>,
}

/// Governance runtime: config, token/usage store, sensitivity detector.
#[derive(Clone)]
pub struct Governance {
    pub config: GovernanceConfig,
    pub store: Arc<dyn Store>,
    pub detector: Detector,
}

pub fn create_app(engine: AppState) -> Router {
    create_app_inner(engine, None)
}

pub fn create_app_with_governance(engine: AppState, governance: Governance) -> Router {
    create_app_inner(engine, Some(governance))
}

fn create_app_inner(engine: AppState, governance: Option<Governance>) -> Router {
    let state = ServerState { engine, governance };

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(openai_chat_completions))
        .route("/v1/messages", post(anthropic_messages))
        .route("/v1/decision", post(decision_only))
        .route("/v1/feedback", post(feedback))
        .route("/metrics", get(prometheus_metrics))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(TimeoutLayer::new(Duration::from_secs(300)))
        .with_state(state)
}
```

In `serve()`, build governance from config when enabled (replace `let app = create_app(engine);`):

```rust
    let governance = if config.governance.enabled {
        let url = config
            .governance
            .redis_url
            .clone()
            .or_else(|| std::env::var("MERLIN_REDIS_URL").ok())
            .unwrap_or_else(|| "redis://127.0.0.1:6379".to_string());
        match crate::governance::apps::RedisStore::connect(&url).await {
            Ok(store) => Some(Governance {
                config: config.governance.clone(),
                store: Arc::new(store),
                detector: Detector::new(),
            }),
            Err(e) => {
                tracing::warn!(
                    "governance enabled but redis at {} unreachable ({}); \
                     governed requests will fail closed with 503",
                    url,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    let app = create_app_inner(engine, governance);
```

Add the governance pipeline types and function (before `openai_chat_completions`):

```rust
/// Outcome of the governance pipeline for one request.
#[derive(Debug, Clone)]
struct GovContext {
    app: App,
    sensitivity: Sensitivity,
    sensitivity_source: &'static str,
    eligible: Vec<String>,
}

/// Per-call gate trace attached to responses as `merlin_trace`.
fn trace_json(ctx: &GovContext, selected: Option<&str>) -> serde_json::Value {
    json!({
        "app": ctx.app.id,
        "sensitivity": match ctx.sensitivity {
            Sensitivity::Clean => "clean",
            Sensitivity::Sensitive => "sensitive",
            Sensitivity::Undetermined => "undetermined",
        },
        "sensitivity_source": ctx.sensitivity_source,
        "eligible": ctx.eligible,
        "selected": selected,
    })
}

fn denial_response(status: StatusCode, code: &str, message: &str, conflicts: Vec<String>) -> Response {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "error": {
                    "message": message,
                    "type": "merlin_error",
                    "code": code,
                    "conflicts": conflicts,
                }
            })
            .to_string(),
        ))
        .unwrap_or_default()
}

/// Run authn + quotas + gate for one request. Returns `Ok(None)` when
/// governance is disabled (anonymous, ungoverned). Returns `Err(response)`
/// when the request is denied — the handler must return it verbatim.
async fn govern_request(
    state: &ServerState,
    headers: &HeaderMap,
    route_name: &str,
    prompt: &str,
) -> Result<Option<GovContext>, Response> {
    let gov = match &state.governance {
        Some(g) if g.config.enabled => g,
        _ => return Ok(None),
    };

    // 1. Authenticate.
    let raw_token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let raw_token = match raw_token {
        Some(t) => t,
        None => {
            state.engine.metrics().record_governance_denial("unauthorized");
            return Err(denial_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "missing bearer token",
                Vec::new(),
            ));
        }
    };
    let hash = hash_token(raw_token);
    let (app_id, _token_id) = gov
        .store
        .app_for_token(&hash)
        .await
        .map_err(|e| {
            tracing::error!("token lookup failed: {e}");
            state.engine.metrics().record_governance_denial("store_error");
            denial_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "store_error",
                "governance store unavailable",
                Vec::new(),
            )
        })?
        .ok_or_else(|| {
            state.engine.metrics().record_governance_denial("unauthorized");
            denial_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "unknown or revoked token",
                Vec::new(),
            )
        })?;
    let app = gov.store.get_app(&app_id).await.map_err(|e| {
        tracing::error!("app lookup failed: {e}");
        state.engine.metrics().record_governance_denial("store_error");
        denial_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "store_error",
            "governance store unavailable",
            Vec::new(),
        )
    })?;
    let app = match app {
        Some(app) => app,
        None => {
            state.engine.metrics().record_governance_denial("unauthorized");
            return Err(denial_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "token references unknown app",
                Vec::new(),
            ));
        }
    };

    // Route allowlist.
    if let Some(allowed) = &app.policy.allowed_routes {
        if !allowed.iter().any(|r| r == route_name) {
            state.engine.metrics().record_governance_denial("policy_denied");
            return Err(denial_response(
                StatusCode::FORBIDDEN,
                "policy_denied",
                &format!("route '{route_name}' not allowed for app '{}'", app.id),
                vec![format!("route '{route_name}' not in allowed_routes")],
            ));
        }
    }

    // 2. Quotas.
    if let Some(max_rpm) = app.policy.max_rpm {
        let minute = chrono::Utc::now().timestamp() / 60;
        let count = gov.store.incr_rpm(&app.id, minute).await.map_err(|e| {
            tracing::error!("rpm counter failed: {e}");
            state.engine.metrics().record_governance_denial("store_error");
            denial_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "store_error",
                "governance store unavailable",
                Vec::new(),
            )
        })?;
        if count > max_rpm as u64 {
            state.engine.metrics().record_governance_denial("rate_exceeded");
            return Err(denial_response(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_exceeded",
                "per-app request rate exceeded",
                Vec::new(),
            ));
        }
    }
    if let Some(cap) = app.policy.max_spend_usd_per_day {
        let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let spent = gov.store.spend(&app.id, &day).await.map_err(|e| {
            tracing::error!("spend lookup failed: {e}");
            state.engine.metrics().record_governance_denial("store_error");
            denial_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "store_error",
                "governance store unavailable",
                Vec::new(),
            )
        })?;
        if spent >= cap {
            state.engine.metrics().record_governance_denial("budget_exhausted");
            return Err(denial_response(
                StatusCode::TOO_MANY_REQUESTS,
                "budget_exhausted",
                "per-app daily spend cap reached",
                Vec::new(),
            ));
        }
    }

    // 3. Sensitivity: sidecar first, local detector fallback.
    let (sensitivity, source) = match state.engine.prompt_features(prompt).await {
        Some(features) => {
            if features.is_sensitive >= gov.config.sensitivity_threshold {
                (Sensitivity::Sensitive, "sidecar")
            } else {
                (Sensitivity::Clean, "sidecar")
            }
        }
        None => match gov.detector.scan(prompt) {
            ScanVerdict::Sensitive => (Sensitivity::Sensitive, "local"),
            ScanVerdict::Clean => (Sensitivity::Clean, "local_degraded"),
            ScanVerdict::Undetermined => (Sensitivity::Undetermined, "undetermined"),
        },
    };

    // 4. Gate.
    let scope: Vec<TargetScope> = state
        .engine
        .route_scope(route_name)
        .into_iter()
        .map(|name| TargetScope {
            local: state.engine.target_locality(&name),
            name,
        })
        .collect();
    let eligible = match gate::evaluate(&app.policy, sensitivity, &scope) {
        Ok(eligible) => eligible,
        Err(denial) => {
            state.engine.metrics().record_governance_denial("policy_denied");
            return Err(denial_response(
                StatusCode::FORBIDDEN,
                "policy_denied",
                "no eligible targets under app policy",
                denial.conflicts,
            ));
        }
    };

    Ok(Some(GovContext {
        app,
        sensitivity,
        sensitivity_source: source,
        eligible,
    }))
}

/// Charge a completed call against the app's daily budget.
async fn charge_spend(state: &ServerState, ctx: Option<&GovContext>, agg: &AggLlmResponse) {
    let (Some(ctx), Some(target_name)) = (ctx, agg.preservation.selected_target.as_ref()) else {
        return;
    };
    // Only track spend when a cap is configured; unpriced targets and
    // uncapped apps skip the store write entirely.
    if ctx.app.policy.max_spend_usd_per_day.is_none() {
        return;
    }
    let Some((price_in, price_out)) = state.engine.target_price(target_name) else {
        return;
    };
    let Some(gov) = state.governance.as_ref() else {
        return;
    };
    let cost = (agg.usage.prompt_tokens as f64) * price_in / 1_000_000.0
        + (agg.usage.completion_tokens as f64) * price_out / 1_000_000.0;
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if let Err(e) = gov.store.add_spend(&ctx.app.id, &day, cost).await {
        tracing::warn!("failed to charge spend for app {}: {e}", ctx.app.id);
    }
}
```

Add the engine accessor (in `src/engine.rs`, with the other methods):

```rust
    /// Sidecar features for a prompt (for the governance pipeline).
    /// `None` when the sidecar is disabled or unreachable.
    pub async fn prompt_features(&self, prompt: &str) -> Option<crate::features::PromptFeatures> {
        self.features.as_ref()?.features(prompt).await
    }
```

Wire the pipeline into `openai_chat_completions` — replace the middle of the handler (after `guard_check`, before `execute`) and the success arm:

```rust
async fn openai_chat_completions(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let codec = crate::translation::openai::OpenAiChatCodec;
    let request = match decode_request(body, &codec) {
        Ok(req) => req,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("bad request: {}", e)),
    };
    if let Err(reason) = state.engine.guard_check(&request).await {
        return error_response(
            StatusCode::BAD_REQUEST,
            &format!("blocked by prompt guard: {}", reason),
        );
    }

    let route_name = infer_route(
        &state.engine,
        request.llm_request.model.as_deref().unwrap_or(""),
    );
    let prompt = request.llm_request.prompt_text();
    let gov = match govern_request(&state, &headers, &route_name, &prompt).await {
        Ok(g) => g,
        Err(response) => return response,
    };
    let eligible = gov.as_ref().map(|g| g.eligible.clone());

    let stream = request.llm_request.stream;
    match state
        .engine
        .execute_governed(&route_name, request, eligible.as_deref())
        .await
    {
        Ok(LlmResponse::Aggregate(agg)) => {
            charge_spend(&state, gov.as_ref(), &agg).await;
            let mut value = openai_aggregate_response(&agg);
            if let Some(g) = &gov {
                value.as_object_mut().expect("object").insert(
                    "merlin_trace".to_string(),
                    trace_json(g, agg.preservation.selected_target.as_deref()),
                );
            }
            Json(value).into_response()
        }
        Ok(LlmResponse::Stream(s)) => {
            if stream {
                sse_stream(s, &codec).into_response()
            } else {
                match crate::engine::aggregate_stream(s).await {
                    Ok(agg) => {
                        charge_spend(&state, gov.as_ref(), &agg).await;
                        let mut value = openai_aggregate_response(&agg);
                        if let Some(g) = &gov {
                            value.as_object_mut().expect("object").insert(
                                "merlin_trace".to_string(),
                                trace_json(g, agg.preservation.selected_target.as_deref()),
                            );
                        }
                        Json(value).into_response()
                    }
                    Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
                }
            }
        }
        Err(e) => error_response(StatusCode::BAD_GATEWAY, &e.to_string()),
    }
}
```

Apply the same pattern to `anthropic_messages` (same pipeline call; `trace_json` inserted into `anthropic_aggregate_response` output) and `decision_only` (pipeline + `decide_governed`; insert trace into the decision JSON).

- [ ] **Step 5: Run tests to verify they pass**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test --test governance_test`
Expected: 5 passed.

Run full suite: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test`
Expected: all passed (existing integration tests included — `create_app` signature unchanged).

- [ ] **Step 6: Commit**

```bash
git add src/metrics/mod.rs src/server.rs src/engine.rs tests/governance_test.rs && git commit -m "feat(server): governance pipeline — auth, quotas, gate, per-call trace"
```

---

### Task 9: `merlin app` CLI

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement the subcommands**

Replace the body of `async_main` in `src/main.rs` — keep the existing `serve` subcommand, add the `app` subcommand tree:

```rust
use clap::{Arg, ArgAction, Command};

fn cli() -> Command {
    Command::new("merlin")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Production-grade multi-provider LLM router")
        .subcommand(
            Command::new("serve")
                .about("Start the HTTP server")
                .arg(
                    Arg::new("port")
                        .long("port")
                        .short('p')
                        .value_name("PORT")
                        .help("Port to listen on (default 7777)"),
                )
                .arg(
                    Arg::new("config")
                        .long("config")
                        .short('c')
                        .value_name("CONFIG")
                        .help("Configuration file path"),
                ),
        )
        .subcommand(
            Command::new("app")
                .about("Manage governed applications")
                .subcommand_required(true)
                .arg(
                    Arg::new("redis")
                        .long("redis")
                        .value_name("URL")
                        .global(true)
                        .help("Redis URL (default: MERLIN_REDIS_URL or redis://127.0.0.1:6379)"),
                )
                .subcommand(
                    Command::new("create")
                        .about("Create an application")
                        .arg(Arg::new("id").long("id").required(true))
                        .arg(Arg::new("name").long("name").required(true)),
                )
                .subcommand(
                    Command::new("policy")
                        .about("Set an application's policy (merges; omitted fields keep current values)")
                        .arg(Arg::new("id").long("id").required(true))
                        .arg(
                            Arg::new("allowed-target")
                                .long("allowed-target")
                                .action(ArgAction::Append)
                                .value_name("TARGET")
                                .help("Restrict to these targets (repeatable)"),
                        )
                        .arg(
                            Arg::new("locality")
                                .long("locality")
                                .value_parser(["any", "local_only"])
                                .help("Locality restriction"),
                        )
                        .arg(
                            Arg::new("max-rpm")
                                .long("max-rpm")
                                .value_parser(clap::value_parser!(u32)),
                        )
                        .arg(
                            Arg::new("max-spend-day")
                                .long("max-spend-day")
                                .value_parser(clap::value_parser!(f64)),
                        )
                        .arg(
                            Arg::new("sensitive-local-only")
                                .long("sensitive-local-only")
                                .action(ArgAction::SetTrue)
                                .help("Raise-only: sensitive requests restricted to local targets"),
                        )
                        .arg(
                            Arg::new("no-sensitive-local-only")
                                .long("no-sensitive-local-only")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("token")
                        .about("Manage app tokens")
                        .subcommand_required(true)
                        .subcommand(
                            Command::new("issue")
                                .about("Issue a token (shown once)")
                                .arg(Arg::new("id").long("id").required(true)),
                        )
                        .subcommand(
                            Command::new("revoke")
                                .about("Revoke a token")
                                .arg(Arg::new("id").long("id").required(true))
                                .arg(Arg::new("token").long("token").required(true).help("token id")),
                        ),
                )
                .subcommand(Command::new("list").about("List applications"))
                .subcommand(
                    Command::new("show")
                        .about("Show one application")
                        .arg(Arg::new("id").long("id").required(true)),
                ),
        )
}

async fn async_main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("serve", sub_matches)) => {
            let config_path = sub_matches
                .get_one::<String>("config")
                .cloned()
                .or_else(|| std::env::var("MERLIN_CONFIG").ok());

            let port = match sub_matches.get_one::<String>("port") {
                Some(p) => p.parse::<u16>()?,
                None => config_path
                    .as_deref()
                    .and_then(|path| merlin::config::MerlinConfig::load_from_file(path).ok())
                    .map(|config| config.server.port)
                    .unwrap_or(7777),
            };

            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            tracing::info!("Starting Merlin server on {}", addr);
            merlin::server::serve(addr, config_path.as_deref()).await?;
        }
        Some(("app", app_matches)) => {
            run_app_command(app_matches).await?;
        }
        _ => {
            eprintln!("Use 'merlin serve --help' or 'merlin app --help' for usage");
        }
    }

    Ok(())
}

async fn run_app_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    use merlin::governance::apps::{App, LocalityPolicy, RedisStore, Store};

    let url = matches
        .get_one::<String>("redis")
        .cloned()
        .or_else(|| std::env::var("MERLIN_REDIS_URL").ok())
        .unwrap_or_else(|| "redis://127.0.0.1:6379".to_string());
    let store = RedisStore::connect(&url).await?;

    match matches.subcommand() {
        Some(("create", sub)) => {
            let id = sub.get_one::<String>("id").expect("required");
            let name = sub.get_one::<String>("name").expect("required");
            if store.get_app(id).await?.is_some() {
                anyhow::bail!("app '{id}' already exists");
            }
            let app = App::new(id, name);
            store.put_app(&app).await?;
            println!("created app '{id}'");
        }
        Some(("policy", sub)) => {
            let id = sub.get_one::<String>("id").expect("required");
            let mut app = store
                .get_app(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("app '{id}' not found"))?;
            if let Some(targets) = sub.get_many::<String>("allowed-target") {
                app.policy.allowed_targets =
                    Some(targets.cloned().collect());
            }
            if let Some(locality) = sub.get_one::<String>("locality") {
                app.policy.locality = match locality.as_str() {
                    "local_only" => LocalityPolicy::LocalOnly,
                    _ => LocalityPolicy::Any,
                };
            }
            if let Some(max_rpm) = sub.get_one::<u32>("max-rpm") {
                app.policy.max_rpm = Some(*max_rpm);
            }
            if let Some(cap) = sub.get_one::<f64>("max-spend-day") {
                app.policy.max_spend_usd_per_day = Some(*cap);
            }
            if sub.get_flag("sensitive-local-only") {
                app.policy.sensitive_local_only = true;
            }
            if sub.get_flag("no-sensitive-local-only") {
                app.policy.sensitive_local_only = false;
            }
            store.put_app(&app).await?;
            println!("updated policy for app '{id}': {:?}", app.policy);
        }
        Some(("token", sub)) => match sub.subcommand() {
            Some(("issue", sub)) => {
                let id = sub.get_one::<String>("id").expect("required");
                if store.get_app(id).await?.is_none() {
                    anyhow::bail!("app '{id}' not found");
                }
                let raw = merlin::governance::apps::mint_token();
                let hash = merlin::governance::apps::hash_token(&raw);
                let token_id = uuid::Uuid::new_v4().to_string();
                store.put_token(id, &token_id, &hash).await?;
                println!("token id:    {token_id}");
                println!("token (shown once, store it now):\n{raw}");
            }
            Some(("revoke", sub)) => {
                let id = sub.get_one::<String>("id").expect("required");
                let token_id = sub.get_one::<String>("token").expect("required");
                store.remove_token(id, token_id).await?;
                println!("revoked token '{token_id}' for app '{id}'");
            }
            _ => unreachable!("subcommand_required"),
        },
        Some(("list", _)) => {
            for app in store.list_apps().await? {
                println!("{}\t{}", app.id, app.name);
            }
        }
        Some(("show", sub)) => {
            let id = sub.get_one::<String>("id").expect("required");
            let app = store
                .get_app(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("app '{id}' not found"))?;
            let tokens = store.token_ids(id).await?;
            println!("id:       {}", app.id);
            println!("name:     {}", app.name);
            println!("created:  {}", app.created_at);
            println!("policy:   {:?}", app.policy);
            println!("tokens:   {}", tokens.join(", "));
        }
        _ => unreachable!("subcommand_required"),
    }
    Ok(())
}
```

(The old `fn cli()`-less inline `Command::new(...)` definition and old match arms are replaced by the above. Keep `fn main()` at the top unchanged.)

- [ ] **Step 2: Build and run unit tests**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test`
Expected: all passed; `cargo build` compiles the binary.

- [ ] **Step 3: Manual smoke test (needs Redis)**

```bash
docker run --rm -d --name merlin-redis -p 6379:6379 redis:7
MERLIN_REDIS_URL=redis://127.0.0.1:6379 \
  CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib \
  cargo run -- app create --id demo --name "Demo App"
MERLIN_REDIS_URL=redis://127.0.0.1:6379 \
  CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib \
  cargo run -- app policy set --id demo --max-rpm 60 --sensitive-local-only
MERLIN_REDIS_URL=redis://127.0.0.1:6379 \
  CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib \
  cargo run -- app token issue --id demo
MERLIN_REDIS_URL=redis://127.0.0.1:6379 \
  CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib \
  cargo run -- app show --id demo
docker stop merlin-redis
```

Expected: create prints `created app 'demo'`; policy prints the updated `AppPolicy`; issue prints a `mrl_live_...` token; show lists policy and token id.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs && git commit -m "feat(cli): merlin app create/policy/token/list/show subcommands"
```

---

### Task 10: Docs, example config, full verification

**Files:**
- Modify: `README.md`
- Create: `examples/governance.toml`

- [ ] **Step 1: Example config**

Create `examples/governance.toml`:

```toml
# Governance-enabled merlin: per-app tokens, quotas, and a fail-closed
# policy gate. Start Redis, create an app + token (see README), then call
# /v1/chat/completions with Authorization: Bearer mrl_live_...

[server]
host = "0.0.0.0"
port = 7777

[clients.ollama]
format = "openai_chat"
base_url = "http://127.0.0.1:11434"

[clients.local_vllm]
format = "openai_chat"
base_url = "http://10.0.0.5:8000/v1"

[targets.local_qwen]
client = "ollama"
model = "qwen3"

[targets.local_vllm]
client = "local_vllm"
model = "qwen3.8-27b"
# USD per 1M tokens — backs daily-spend accounting for governed apps.
price_input = 0.0
price_output = 0.0

[routes.default]
id = "default"
targets = ["local_qwen", "local_vllm"]
[routes.default.algorithm]
type = "thompson"

[governance]
enabled = true
redis_url = "redis://127.0.0.1:6379"
sensitivity_threshold = 0.7
```

- [ ] **Step 2: README section**

Append a "Governance" section to `README.md` before the Roadmap section:

```markdown
## 🛡️ Governance Suite (per-app policy gate)

Governance turns merlin into a multi-tenant policy gateway: applications
authenticate with `mrl_live_…` tokens, per-app policies restrict which
targets may serve them, quotas cap requests-per-minute and daily spend,
and a fail-closed gate keeps sensitive traffic on local infrastructure.

```bash
# 1. Enable in merlin.toml
[governance]
enabled = true
redis_url = "redis://127.0.0.1:6379"

# 2. Create an app and issue a token (shown once)
merlin app create --id myapp --name "My App"
merlin app policy set --id myapp --locality local_only --max-rpm 60 \
    --max-spend-day 20 --sensitive-local-only
merlin app token issue --id myapp

# 3. Call with the token
curl -X POST http://localhost:7777/v1/chat/completions \
  -H "Authorization: Bearer mrl_live_…" \
  -H "Content-Type: application/json" \
  -d '{"model": "default", "messages": [{"role": "user", "content": "hello"}]}'
```

Policy enforcement order: authenticate (401) → quotas (429) → sensitivity
(sidecar `is_sensitive`, falling back to a built-in local detector) →
eligibility gate (403 with self-explaining conflicts). When sensitivity
cannot be determined the gate fails **closed** and applies the strictest
policy; routing-side classification stays fail-open as before. Successful
responses include a `merlin_trace` object recording the decision.

Denials are counted in `merlin_governance_denials_total{code}`.
```

- [ ] **Step 3: Full verification**

Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo fmt --check`
Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo clippy --all-targets`
Run: `CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib cargo test`
Expected: fmt clean (run `cargo fmt` first if not), clippy no new warnings, all tests passed.

- [ ] **Step 4: Commit**

```bash
git add README.md examples/governance.toml && git commit -m "docs: governance suite usage and example config"
```

---

## Self-Review Notes

- **Spec coverage:** detect+sidecar fallback (Tasks 2, 8), Redis store + in-memory test impl (Tasks 3–4), gate fail-closed semantics (Task 5), property test (Task 6), eligible-only bandit selection + algorithm-escape guard (Task 7), auth/quota/gate/trace/denial metrics (Task 8), CLI (Task 9), config surface + pricing (Task 1). All spec sections map to tasks.
- **Type consistency:** `AppPolicy`/`LocalityPolicy`/`App`/`Store`/`MemoryStore`/`RedisStore`/`mint_token`/`hash_token` (Task 3) are used with identical signatures in Tasks 4–9. `evaluate(policy, sensitivity, scope) -> Result<Vec<String>, Denial>` and `Denial { code, conflicts }` are used identically in Tasks 5, 6, 8. `execute_governed(route, request, Option<&[String]>)` / `decide_governed` same in Tasks 7, 8. `Governance { config, store, detector }` and `create_app_with_governance` identical in Tasks 8–9 tests.
- **Known v1 limitations (documented in spec, out of scope):** spend is charged on aggregate responses only (SSE passthrough uncharged); no payload redaction; no admin HTTP API.
- **Task 5 execution notes:** three plan defects were fixed during
  execution — `host_is_local` gained bracket-aware IPv6 extraction (the
  reference parser failed on `[::1]` against its own test), denial wording
  adjusted so the plan's asserted substring matches, and
  `allowlist_and_locality_compose`'s allowlist was widened to
  `["cloud", "local-b"]` (as written, no implementation could satisfy it).
  `host_is_local` was further hardened to parse IPs via
  `std::net::IpAddr` (fail-closed; hostname prefix matching was fail-open
  on e.g. `127.0.0.1.evil.com`).
- **Task 8 execution notes:** the test helper now applies its `policy`
  parameter to the stored app, and the deny test sets
  `sensitive_local_only: true`; engine-level denials are typed
  (`GovernanceError`) and surface as 403 `policy_denied` with a metric.
