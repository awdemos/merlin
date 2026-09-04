# Merlin Refactor Inspection Report

Inspection date: 2026-09-03
Commit: `d9a1aa4` (and merge `f24ccfe` on `main`)
Scope: `src/`, `tests/integration_test.rs`, `Cargo.toml`, `merlin.toml`

## Executive summary

The refactor introduces clean module boundaries, a provider-neutral protocol, multiple bandit routing algorithms, and OpenAI/Anthropic wire translation. The code compiles, passes clippy, and the integration tests pass. The server starts and the health, models, metrics, and decision endpoints respond correctly.

However, several production-looking features are stubs or config flags routing around incomplete implementations. The biggest risks are:

1. **Security/ops:** permissive CORS, unvalidated config file paths, no URL scheme validation, API keys read from arbitrary env-var names, and the server binding `0.0.0.0` by default.
2. **Correctness:** `/v1/feedback` is a no-op, `timeout_ms` config is ignored, metrics labels are computed then discarded, contextual bandit/classifier routing are placeholders, and unknown wire formats silently fall back to OpenAI.
3. **Maintainability:** duplicated tool-call decoding, duplicated fallback-list construction, ad-hoc JSON building in each codec, and config loaded twice.
4. **Dead weight:** several declared dependencies and two stub modules (`quantum`, `swarm`) are not used.

## Security findings (rust-cli-security-review)

| # | Location | Issue | Risk | Suggested fix |
|---|----------|-------|------|---------------|
| S1 | `src/server.rs:52-55` | CORS allows any origin, method, header | CSRF / information leak in production | Default to same-origin or make `allow_origin` configurable; never default to `Any` |
| S2 | `src/main.rs:44`, `src/server.rs:33-34` | Config path taken from CLI or `MERLIN_CONFIG` env and passed straight to `std::fs::read_to_string` | Path traversal / symlink attack on config | Resolve to an absolute path, validate it is a regular file owned by the user, refuse symlinks unless opted in |
| S3 | `src/clients/translating.rs:53`, `src/engine.rs:182` | `base_url` is a plain `String`; joined by string concat to build upstream URL | SSRF / open redirect via `file://`, `http://internal`, or missing host | Parse with `url::Url`, reject non-http(s) schemes and URLs with credentials, validate host |
| S4 | `src/clients/translating.rs:60-63` | API key env-var name comes from config; reads any env var without validation | Accidental secret exfiltration / confused-deputy | Restrict env names to an allow-list or require keys in a dedicated secrets file; validate the env var name format |
| S5 | `src/config.rs:21-23` | Server defaults to `0.0.0.0:7777` | Exposes debug server on all interfaces | Default to `127.0.0.1` unless explicitly configured otherwise |
| S6 | `src/clients/translating.rs:38-39` | `extra_headers` are built with `HeaderName::from_bytes`/`HeaderValue::from_str` without filtering | Header injection / request splitting | Validate header names against an allow-list; reject `Host`, `Content-Length`, etc. |
| S7 | `src/translation/openai_decode.rs:109`, `:170` | Image URLs and base64 payloads from inbound requests are forwarded upstream unchecked | SSRF via image URL, data exfiltration via base64 | Validate URL scheme/host for image URLs; cap base64 size |
| S8 | `src/server.rs:67` | Server timeout hard-coded at 300s | DoS via long-lived connections | Tie to config; set a sane ceiling |
| S9 | `src/clients/translating.rs:43-44` | `timeout_ms` config ignored; reqwest timeout hard-coded 120s | Misleading config / resource exhaustion | Use `Duration::from_millis(config.timeout_ms)` |
| S10 | `src/metrics/mod.rs:52-68` | Labels built then discarded (`let _ = labels`) | Unusable per-route/target metrics, debugging blind spots | Use `CounterVec`/`HistogramVec` with route/target labels |

## Correctness / altitude findings (simplify-code review)

| # | Location | Issue | Suggested fix |
|---|----------|-------|---------------|
| A1 | `src/server.rs:204-219` | `/v1/feedback` parses payload but does not update any algorithm | Route reward to `RouterEngine` and call `record_reward` on the route's algorithm |
| A2 | `src/clients/translating.rs:43-44` | `timeout_ms` config ignored; hard-coded 120s | Use `config.timeout_ms` for reqwest `.timeout()` |
| A3 | `src/metrics/mod.rs:52-68` | Labels computed then discarded | Use labeled Prometheus metrics |
| A4 | `src/routing/contextual.rs:8-11,35-39` | Contextual bandit is a deterministic hash placeholder | Implement feature-based scoring or reject the config at validation time |
| A5 | `src/routing/classifier.rs:10-26,37-38` | Classifier target and `base_threshold` ignored; routes by prompt length | Use the configured threshold and invoke the classifier, or remove the feature |
| A6 | `src/config.rs:54-76`, `src/translation/mod.rs:32-39` | Unknown wire format silently falls back to OpenAI | Fail validation on unknown format; do not `unwrap_or` |
| A7 | `src/config.rs:178-199` | Algorithm target list duplicated alongside `route.targets` | Single source of truth for route targets |
| A8 | `src/server.rs:33-37`, `src/main.rs:47-53` | Config loaded twice to resolve port | Load once in `main.rs`, pass resolved `MerlinConfig` plus CLI overrides |
| A9 | `src/engine.rs:188-227` | Request model override duplicated in fallback loop | Extract `build_upstream_request(request, target)` helper |
| A10 | `src/translation/anthropic.rs:17-67`, `src/translation/openai.rs:21-73` | Ad-hoc JSON construction per codec | Typed intermediate request DTO per wire format |
| A11 | `src/protocol/conversation.rs:158-177` | `prompt_text()` re-allocates on every routing call | Cache prompt text or add feature extractor |
| A12 | `src/quantum/mod.rs`, `src/swarm/mod.rs` | Unused stub modules exported | Feature-gate or remove |
| A13 | `src/config.rs:129-146` | Empty target/route names silently backfilled | Fail validation or document behavior |
| A14 | `src/translation/openai.rs:89`, `:172` | Tool-call decoding duplicated in response and SSE paths | Extract `decode_tool_calls` helper |
| A15 | `src/routing/epsilon_greedy.rs`, `thompson.rs`, `ucb.rs`, `random.rs`, `contextual.rs` | Fallback list rebuilt inline in every algorithm | Use `crate::routing::fallback_targets` or a shared helper |

## Efficiency findings (simplify-code review)

| # | Location | Issue | Suggested fix |
|---|----------|-------|---------------|
| E1 | `src/engine.rs:98-103`, `:155-160` | `execute`/`decide` clone every `TargetConfig` into a `Vec` | Use references or indices into the config map |
| E2 | `src/routing/mod.rs:41-47` | `fallback_targets` clones every non-selected target config | Return names or references |
| E3 | `src/routing/contextual.rs:39` | `prompt_text()` bytes summed for hashing | Use a cheaper hash/fingerprint if this remains a placeholder |
| E4 | `Cargo.toml` | Multiple declared dependencies are unused in source: `hyper`, `chrono`, `uuid`, `base64`, `thiserror`, `rand_distr`, `redis`, `opentelemetry*`, `tracing-opentelemetry` | Remove unused deps to cut compile time and binary size (current release binary ~13 MB) |

## Build / test verification

- `cargo build` ✅
- `cargo clippy` ✅
- `cargo test` ✅ (3 integration tests)
- `cargo fmt --check` ✅
- Release binary size: ~13 MB

## Recommendations (priority order)

1. **Fix security defaults before any production use:** CORS, config path validation, URL validation, server bind address, header filtering.
2. **Make config truthful:** reject unknown wire formats, reject contextual/classifier until implemented, or implement them.
3. **Connect feedback to learning:** `/v1/feedback` should update the selected route's algorithm reward.
4. **Use the timeout config:** `timeout_ms` is currently dead data.
5. **Clean up dependencies:** remove crates not used in source/tests; this also removes the redis future-incompat warning surface.
6. **Refactor duplicated codec/routing code:** shared helpers for tool-call decoding, fallback lists, and upstream request building.
7. **Prometheus labels:** switch to `CounterVec`/`HistogramVec`.
8. **Remove or feature-gate `quantum` and `swarm` stubs.**

## Files referenced

- `src/config.rs`
- `src/engine.rs`
- `src/server.rs`
- `src/clients/translating.rs`
- `src/metrics/mod.rs`
- `src/routing/contextual.rs`
- `src/routing/classifier.rs`
- `src/routing/mod.rs`
- `src/routing/epsilon_greedy.rs`
- `src/routing/thompson.rs`
- `src/routing/ucb.rs`
- `src/routing/random.rs`
- `src/translation/openai.rs`
- `src/translation/anthropic.rs`
- `src/translation/openai_decode.rs`
- `src/translation/mod.rs`
- `src/protocol/conversation.rs`
- `src/quantum/mod.rs`
- `src/swarm/mod.rs`
- `src/main.rs`
- `Cargo.toml`
- `merlin.toml`
- `tests/integration_test.rs`

---

*Generated by parallel `simplify-code` review plus `rust-cli-security-review`.*
