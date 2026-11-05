# Merlin Governance Suite — Design

**Date:** 2026-11-05
**Status:** Approved (design)
**Motivation:** Close the competitive capability gaps identified in the
Merlin vs. RouteForge analysis (policy/eligibility layer, per-app tokens,
quotas/budgets, fail-closed governance) while preserving Merlin's
fail-open routing philosophy everywhere else.

## Decisions (user-approved)

| Question | Decision |
|---|---|
| Scope | Governance suite (policy gate + per-app tokens + quotas/budgets) |
| Sensitivity signal | Laya sidecar + built-in Rust fallback detector |
| Storage | Redis (tokens hashed; usage counters with TTL windows) |
| Admin surface | CLI subcommands only (no admin HTTP API in v1) |
| Fail mode | Gate closed, routing open (undetermined sensitivity ⇒ strictest policy; routing-side classification outages stay fail-open) |
| Architecture | Gate inside `RouterEngine` as a pre-bandit stage; server only does token→app authn |

## Architecture

New module `src/governance/` with three single-purpose units. Request
pipeline becomes: **authenticate → quota → gate (eligibility) → bandit
selection over eligible arms only → upstream call → response**.

### `governance::apps` — identity & policy store

- `App { id, name, policy, created_at }`.
- `AppPolicy`:
  - `allowed_routes: Option<Vec<String>>` — client-visible route allowlist (`None` = all).
  - `allowed_targets: Option<Vec<String>>` — upstream target allowlist (`None` = all).
  - `locality: LocalityPolicy` — `Any` | `LocalOnly`.
  - `max_rpm: Option<u32>`.
  - `max_spend_usd_per_day: Option<f64>`.
  - `sensitive_local_only: bool` — raise-only sensitivity override.
- Tokens: `mrl_live_<random>`; only SHA-256 hash stored (shown once at issue).
- Usage counters in Redis, atomic: per-minute request count (key TTL 60 s),
  per-day spend (key TTL 24 h).
- `Store` trait with a Redis implementation and an in-memory implementation
  (tests stay hermetic; no live Redis required for `cargo test`).

### `governance::detect` — local sensitivity fallback

- Deterministic Rust detector, no heavy deps: regex patterns (SSN, credit
  card, email, phone, API-key shapes) + Shannon-entropy scan for
  high-entropy secrets.
- Three-state verdict: `Sensitive` | `Clean` | `Undetermined`.
  `Undetermined` is produced **only** by detector error, never by "no
  patterns matched" (that is `Clean`).

### `governance::gate` — the eligibility seam

- Pure function: `(app.policy, sensitivity) -> eligible target set`.
- Sensitivity resolution order:
  1. Sidecar `is_sensitive` probability (cached via existing features client).
  2. Sidecar unavailable → local detector verdict.
  3. Detector error → `Undetermined`.
- `Undetermined` ⇒ strictest policy (fail-closed gate).
- `Sensitive` + `sensitive_local_only` ⇒ local targets only (raise-only;
  never lowers an app's posture).
- "Local target" derived from config: the target's client `base_url` parses
  to a host that is `localhost`, loopback (`127.0.0.0/8`, `::1`), or a
  private-range IP (`10/8`, `172.16/12`, `192.168/16`, `169.254/16`).
  Anything else (public IPs, public DNS names) is non-local.
- Empty eligible set ⇒ request rejected, never falls through to a
  non-eligible endpoint.

### Engine integration

- `RouterEngine` gains a governed path: bandit selection receives the
  eligible arm list instead of the full route target list.
- Bandit algorithms and the Verus-verified core (`src/routing/bandit_verified.rs`)
  are untouched — eligibility is arm filtering before scoring.

## Data flow (governance enabled)

```
Authorization: Bearer mrl_live_…
        │
        ▼
 1. Authenticate  ──unknown/revoked──▶ 401
        │
        ▼
 2. Quotas  ──over RPM───────────────▶ 429 {code:"rate_exceeded"}
           ──over daily spend───────▶ 429 {code:"budget_exceeded"}
        │
        ▼
 3. Resolve sensitivity (sidecar → local detect → Undetermined)
        │
        ▼
 4. Gate: eligible = policy ∩ sensitivity
   ──empty set──▶ 403 {code:"policy_denied", conflicts:[…]}
        │
        ▼
 5. Bandit selection over eligible arms → upstream → response
```

## Error handling

- **Sidecar down, detector `Clean`:** proceed under normal policy; trace
  records `sensitivity_source: "local_degraded"`.
- **Detector error (`Undetermined`):** strictest policy applied. If the
  strict set is empty ⇒ 403 (fail-closed seam).
- **Redis unreachable on the request path:** 503 when a token is presented.
  When governance is disabled there is no new Redis dependency.
- **Spend accounting:** `TargetConfig` gains optional `price_input` /
  `price_output` (USD per 1M tokens). After a call completes, the app is
  charged `input_tokens × price_input + output_tokens × price_output`
  against the daily budget. Targets without prices are not charged
  (documented; operators self-hosting local models can leave them unset).
- **Denial bodies name the conflict,** e.g.
  `{"error":{"code":"policy_denied","conflicts":["sensitive_local_only","no local targets in route"]}}`.
  Hints must never suggest relaxing a security requirement.
- Routing-side classification (domain/difficulty features for bandits) keeps
  its existing fail-open behavior. Only the gate is fail-closed.

## Observability

- `merlin_governance_denials_total{code}` Prometheus counter.
- `GateTrace { app, sensitivity, sensitivity_source, eligible, denied, reason }`
  attached to responses as `merlin_trace` (JSON body) and logged per call.

## Config

New `[governance]` section in `merlin.toml`:

```toml
[governance]
enabled = false                # default; existing deployments unaffected
sensitivity_threshold = 0.7    # sidecar is_sensitive probability cutoff
```

Locality is derived from client `base_url` (loopback/private ⇒ local), not
separately configured. App policies live in Redis (CLI-managed), not TOML.
`TargetConfig` gains optional `price_input` / `price_output` (USD per 1M
tokens) to back daily-spend accounting (see Error handling).

## CLI

```
merlin app create --id myapp --name "My App"
merlin app policy set --id myapp \
    --locality local_only --max-rpm 60 --max-spend-day 20 --sensitive-local-only
merlin app token issue --id myapp        # prints mrl_live_… once
merlin app token revoke --id myapp --token <id>
merlin app list
merlin app show --id myapp
```

## Testing

- **Unit:** `detect` patterns/entropy (incl. negative cases), gate
  eligibility over the full policy×sensitivity matrix, token hashing.
- **Property test (proptest — one small new dep):** for any policy and
  sensitivity, the eligible set never contains a forbidden target.
- **Integration:** in-memory `Store`; existing wiremock-style tests extended
  with governed-route cases — 401/403/429 paths, sidecar-down fail-closed,
  raise-only sensitivity, detector-clean-degraded path.

## Explicitly out of scope (v1)

- Admin HTTP API / console UI
- Payload redaction (v1 restricts *where* prompts go; it does not rewrite them)
- Agent trajectory envelopes
- Policy replay simulator
- Changes to streaming behavior
- Decision-trace pipeline beyond the lightweight `GateTrace`

## Environment note

In this workspace, builds/tests need
`CARGO_TARGET_DIR=target LIBRARY_PATH=/var/home/a/.linuxbrew/lib` prefixes
(workspace `.cargo/config.toml` redirects target-dir to an unwritable path).
