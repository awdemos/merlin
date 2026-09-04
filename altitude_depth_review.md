# Altitude/Depth Review — Merlin refactor (d9a1aa4)

Scope: recent production-grade refactor on `main` (HEAD~1 → HEAD). Focused on band-aids layered on shared infrastructure, special cases in generic paths, symptom patches at call sites, wrappers dodging real fixes, and config flags routing around broken defaults.

---

## Findings

### 1. `src/clients/translating.rs:43-44` → hard-coded `DEFAULT_TIMEOUT` ignores per-client `timeout_ms` config
**Problem:** `ClientConfig` carries a `timeout_ms` field, and `TargetRef` also exposes `timeout_ms`, but `TranslatingLlmClient::new` builds the reqwest client with a hard-coded 120s `DEFAULT_TIMEOUT` and never uses `config.timeout_ms`. The configured value is dead data that looks operational.
**Cost:** Users cannot actually tune upstream timeouts, while the surrounding timeout machinery (config, target ref field) pretends they can. Large requests can silently block the pool; small-timeout SLOs cannot be enforced.
**Suggested fix:** Drive reqwest `.timeout()` from `Duration::from_millis(config.timeout_ms)`. Remove the unused `timeout_ms` from `TargetRef` if it is not plumbed.
**confidence: high | risk: SAFE**

---

### 2. `src/metrics/mod.rs:52-68` → Prometheus metrics discard structured labels
**Problem:** `record_request`, `record_tokens`, and `record_fallback` all build or receive route/target identifiers, then discard them (`let _ = labels`). Counter and histogram metrics are therefore unlabeled — you cannot tell which route or target is failing, falling back, or consuming tokens.
**Cost:** The metrics module is a wrapper around Prometheus that hides Prometheus’s key capability. It is a band-aid: it looks like production metrics but collapses dimensions to global counters, so operators cannot root-cause per-provider behavior.
**Suggested fix:** Register labeled metrics (`CounterVec`, `HistogramVec`) with `target` and `route` labels, or accept that labels are essential and wire them. If label cardinality is a concern, document a whitelist strategy.
**confidence: high | risk: SAFE**

---

### 3. `src/server.rs:204-219` → `/v1/feedback` endpoint is a no-op
**Problem:** The feedback endpoint parses JSON, reads a `reward`, extracts a `route_name`, then does nothing with the data beyond logging. The comment in `contextual_tests.rs:200` explicitly references this prior no-op behavior, suggesting it was patched at the call site rather than fixed in the feedback/reward pipeline.
**Cost:** Reinforcement-learning routers (Epsilon-Greedy, UCB, Thompson, Contextual) cannot learn from external feedback despite the API existing. It is a symptom patch: the endpoint exists to satisfy an interface but does not feed the engine.
**Suggested fix:** Route `feedback` into `RouterEngine` and call `record_reward` on the route’s algorithm with the provided reward. If persistence or async processing is needed, queue the update instead of dropping it.
**confidence: high | risk: SAFE**

---

### 4. `src/routing/contextual.rs:8-11` and `:35-39` → ContextualBandit is a placeholder wired into production routing
**Problem:** `learning_rate` and `exploration_rate` are decorated with `#[allow(dead_code)]`; selection hashes prompt bytes to an index. A configured contextual bandit is silently deterministic and context-blind.
**Cost:** Operators enabling `type = "contextual"` believe they are running a contextual bandit; in reality they get a checksum-based picker. The `#[allow(dead_code)]` is a band-aid suppressing the symptom (unused fields) rather than implementing the feature.
**Suggested fix:** Either implement actual feature-based arm scoring, or fail validation for `Contextual` with an explicit "not yet implemented" error so the config does not lie. Remove `#[allow(dead_code)]` by using the parameters or deleting them.
**confidence: high | risk: SAFE**

---

### 5. `src/routing/classifier.rs:10-26` and `:37-38` → LlmClassifier ignores the configured classifier target and threshold
**Problem:** The struct stores `_classifier` and a `threshold`, but `new` hard-codes `threshold: 0.5` and `select` never calls the classifier target. Instead it uses prompt length as a proxy.
**Cost:** A route configured with a dedicated classifier model and a custom threshold still routes by prompt length. The `_classifier` field is preserved only to avoid warnings; the `base_threshold` config value is ignored after being parsed.
**Suggested fix:** Use `base_threshold` from config (propagate through `new`) and either invoke the classifier target or remove the LLM-as-classifier feature from config until it is real. Do not keep a target that is never called.
**confidence: high | risk: SAFE**

---

### 6. `src/config.rs:54-76` and `src/translation/mod.rs:32-39` → wire-format parsing silently defaults unknown values
**Problem:** `ClientConfig::format()` and `TargetConfig::format()` both call `WireFormat::from_str(...).unwrap_or(WireFormat::OpenAiChat)`. `translation::codec_for` does the same fallback to OpenAI.
**Cost:** A typo in config (`format = "anthropic_message"`) silently routes traffic through the OpenAI codec, producing opaque upstream errors or leaking wrong payloads. This is a band-aid that converts a clear validation failure into a confusing runtime failure.
**Suggested fix:** Validate `format` during `MerlinConfig::validate()` and fail fast on unknown values. The shared `WireFormat::from_str` should stay strict; callers should not paper over errors.
**confidence: high | risk: SAFE**

---

### 7. `src/config.rs:178-199` → `AlgorithmConfig::target_names()` duplicates route target validation logic
**Problem:** `RouteConfig` already has a `targets` field, and each algorithm variant repeats its own target list. `validate()` checks both lists independently. The lists can drift, and `target_names()` needs bespoke matching per variant.
**Cost:** This is a special case in generic validation: instead of a single source of truth for which targets a route uses, there are two. Mismatches (e.g., `route.targets` updated without `route.algorithm.targets`) pass validation but break routing.
**Suggested fix:** Make the algorithm derive its target names from `route.targets` by index/selection, or remove `route.targets` and let the algorithm own the list. Keep exactly one list.
**confidence: medium | risk: CAREFUL** (requires config schema change)

---

### 8. `src/server.rs:33-37` and `src/main.rs:47-53` → port resolution loads config twice and conflates CLI/config precedence
**Problem:** `main.rs` parses `--port`; if absent, it loads the config file merely to read `server.port`, then passes the path to `server::serve`, which loads the same config again. There is no single authoritative load point.
**Cost:** Config is parsed twice, error messages may appear at different lifecycle stages, and the precedence rule is scattered across two files. This is a call-site workaround for not having a unified config-resolution function.
**Suggested fix:** Load config once in `main.rs`, pass the resolved `MerlinConfig` (and CLI overrides) into `server::serve`. Let `--port` override `config.server.port` explicitly.
**confidence: medium | risk: SAFE**

---

### 9. `src/engine.rs:188-227` → fallback loop manually clones request and mutates `req.llm_request.model`
**Problem:** Each fallback iteration clones the whole request and overwrites `model`. The model override logic is duplicated in the primary path (line 95/153) and the fallback path (line 224), and it relies on `TargetRef.model_id` being set correctly.
**Cost:** Two call sites mutate the same field for the same reason. A missing or stale `model_id` silently sends the wrong model name upstream. This is a symptom patch rather than a request-build abstraction.
**Suggested fix:** Extract a `build_upstream_request(request, target)` helper that applies model override, extra headers, and target-specific body once per dispatch. Use it for primary and fallbacks.
**confidence: medium | risk: SAFE**

---

### 10. `src/server.rs:67` → server-layer `TimeoutLayer` is fixed at 300s with no config tie-in
**Problem:** Axum gets a hard-coded `TimeoutLayer::new(Duration::from_secs(300))`, while upstream timeouts are 120s (and intended to be configurable). End-to-end timeout is therefore disconnected from upstream timeout.
**Cost:** A request can hit the server 300s ceiling while the upstream client already gave up, producing inconsistent error surfaces. This is a wrapper around missing unified timeout policy.
**Suggested fix:** Drive server timeout from config (e.g., a global `request_timeout_ms`) and ensure it is at least upstream timeout + margin, or remove it if the engine handles timeouts.
**confidence: medium | risk: SAFE**

---

### 11. `src/translation/anthropic.rs:17-67` and `src/translation/openai.rs:21-73` → codec encode bodies built by ad-hoc `serde_json::json!` merge
**Problem:** Both encoders hand-roll JSON construction: base fields, then sampling/output blocks, then extra_body. Shared concepts (sampling, output, tools) are re-implemented per codec.
**Cost:** Adding a new field means editing two or more codec files consistently. This is a band-aid over a provider-neutral request representation: the generic `LlmRequest` is not serialized cleanly; each codec reconstructs the wire shape.
**Suggested fix:** Introduce a typed intermediate request DTO per wire format so encoding is `serde`-driven and provider-specific overrides are limited to a small transform layer.
**confidence: medium | risk: CAREFUL**

---

### 12. `src/protocol/conversation.rs:158-177` → `prompt_text()` O(n) string concatenation used in hot path
**Problem:** `prompt_text()` builds a new `String` by walking instructions and messages every time it is called. `ContextualBandit::select` and `LlmClassifier::select` call it per request.
**Cost:** For large conversations this is a repeated allocation. More importantly, it is a workaround for not having a feature vector / prompt embedding abstraction in the routing layer.
**Suggested fix:** Keep prompt text cached on `LlmRequest` at construction/decode time, or expose a lightweight feature extractor interface for routing.
**confidence: low | risk: SAFE** (performance, not altitude)

---

### 13. `src/quantum/mod.rs` and `src/swarm/mod.rs` → unused experimental modules kept in lib root
**Problem:** Both modules are stubbed and exported from `lib.rs` but never used by the engine. They introduce unused dependencies in the conceptual architecture.
**Cost:** They are architectural band-aids: kept around to signal future direction while doing nothing, increasing surface area and compile times. Unless they are actively integrated, they route around a focused core.
**Suggested fix:** Move them behind a Cargo feature (`experimental`) or remove from `lib.rs` until they participate in routing.
**confidence: low | risk: SAFE**

---

### 14. `src/config.rs:129-146` → `load_from_file` mutates names from empty to map key as a side effect
**Problem:** The loader silently backfills `target.name` and `route.id` if they are empty. This is a compatibility shim that papers over config authoring mistakes.
**Cost:** It makes validation appear more lenient than the schema implies. Downstream code assumes `name`/`id` are meaningful; empty defaults would break routing. This is a symptom patch at the loader rather than a strict schema.
**Suggested fix:** Either make `name`/`id` required (fail validation when empty) or document and test this defaulting behavior explicitly. Prefer failing fast.
**confidence: medium | risk: SAFE**

---

## Summary table

| # | file:line | problem | cost | fix | confidence | risk |
|---|-----------|---------|------|-----|------------|------|
| 1 | `src/clients/translating.rs:43-44` | `timeout_ms` config ignored, hard-coded 120s | false timeout control | use `config.timeout_ms` | high | SAFE |
| 2 | `src/metrics/mod.rs:52-68` | labels built then discarded | unusable per-target metrics | labeled counters/histograms | high | SAFE |
| 3 | `src/server.rs:204-219` | feedback endpoint no-op | RL algorithms cannot learn | route reward to engine | high | SAFE |
| 4 | `src/routing/contextual.rs:8-11,35-39` | contextual bandit is deterministic hash placeholder | fake algorithm | implement or reject config | high | SAFE |
| 5 | `src/routing/classifier.rs:10-26,37-38` | classifier target and threshold ignored | wrong routing logic | use threshold + call classifier | high | SAFE |
| 6 | `src/config.rs:54-76`, `src/translation/mod.rs:32-39` | unknown wire format silently falls back | wrong codec, opaque errors | validate strictly | high | SAFE |
| 7 | `src/config.rs:178-199` | algorithm target list duplicated | drift, double validation | single source of truth | medium | CAREFUL |
| 8 | `src/server.rs:33-37`, `src/main.rs:47-53` | config loaded twice for port | double parse, scattered precedence | load once, pass config | medium | SAFE |
| 9 | `src/engine.rs:188-227` | model override duplicated in fallback loop | stale overrides, drift | extract request-build helper | medium | SAFE |
| 10 | `src/server.rs:67` | server timeout hard-coded 300s | inconsistent with upstream | tie to config | medium | SAFE |
| 11 | `src/translation/anthropic.rs:17-67`, `src/translation/openai.rs:21-73` | ad-hoc JSON construction per codec | duplicated, error-prone | typed DTO per format | medium | CAREFUL |
| 12 | `src/protocol/conversation.rs:158-177` | `prompt_text()` re-allocates | repeated work | cache or feature extractor | low | SAFE |
| 13 | `src/quantum/mod.rs`, `src/swarm/mod.rs` | unused stub modules | dead surface area | feature-gate or remove | low | SAFE |
| 14 | `src/config.rs:129-146` | empty names auto-filled | lenient schema | fail fast or document | medium | SAFE |

---

## Overall verdict

The refactor introduces clean module boundaries but several production-looking features are placeholders or config flags routing around incomplete implementations. The highest-altitude band-aids are the no-op feedback endpoint, the labeled-but-discarded metrics, the contextual/classifier routing stubs, and the hard-coded/ignored timeout values. These are not style issues; they are features that appear to work but do not.
