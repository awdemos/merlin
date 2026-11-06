<div align="center">

# 🧙‍♂️ Merlin

### Intelligent Multi-Provider LLM Router

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)](https://www.gnu.org/licenses/gpl-3.0)
[![Tokio](https://img.shields.io/badge/Tokio-000000?style=for-the-badge&logo=rust&logoColor=white)]()

**Sub-millisecond LLM prompt routing using reinforcement learning.**  
Route every prompt to the optimal model based on latency, cost, and quality — with zero GC pauses.

Built for [RegicideOS](https://github.com/awdemos/RegicideOS) and production AI infrastructure.

[⚡ Quick Start](#quick-start) · [🧠 Routing Algorithms](#routing-algorithms) · [📊 Observability](#observability) · [🚀 Deployment](#deployment)

</div>

---

## 🎯 What Merlin Does

Merlin sits between your application and multiple LLM providers. Instead of hard-coding a model choice, you send a prompt to Merlin and it **intelligently selects the best provider in real time** based on:

- ⚡ **Latency** — Route to the fastest available endpoint
- 💰 **Cost** — Prefer cheaper models for low-stakes queries
- 🎯 **Quality** — Use premium models when accuracy matters
- 📈 **Load** — Balance traffic across healthy providers

All routing decisions happen in **under a millisecond** thanks to Rust + Tokio async.

---

## ✨ Features

| Feature | Detail |
|---------|--------|
| 🔌 **Multi-Provider** | OpenAI, Anthropic, Gemini, local GGUF models — all via a single endpoint |
| 🧠 **Smart Routing** | Epsilon-greedy, Thompson sampling, UCB, and contextual bandit algorithms |
| 📊 **Real-Time Metrics** | Latency, cost, and quality tracking per provider |
| 🔄 **Learning from Feedback** | Star ratings + comment sentiment feed the bandit reward; request success/failure rewards are automatic |
| 📈 **Observability** | Prometheus metrics, Jaeger tracing, Grafana dashboards |
| 🦀 **Zero GC Pauses** | Rust + Tokio async for predictable, sub-ms routing |
| 🔒 **Security Hardened** | Systemd service with non-root execution, memory limits, auto-restart |

---

## ⚡ Quick Start

```bash
# Clone
git clone https://github.com/awdemos/merlin.git
cd merlin

# Build
cargo build --release

# Configure
cat > merlin.toml << 'EOF'
[providers]
openai = { api_key = "sk-...", model = "gpt-4-turbo" }
anthropic = { api_key = "sk-ant-...", model = "claude-3-opus" }

[routing]
policy = "thompson_sampling"
exploration_rate = 0.15

[telemetry]
prometheus_port = 9090
EOF

# Run
./target/release/merlin serve --port 7777 --config ./merlin.toml
```

---

## 🧠 Routing Algorithms

Merlin implements several multi-armed bandit strategies for provider selection:

### Epsilon-Greedy
Explores random providers with probability ε, otherwise exploits the current best.

```toml
[routing]
policy = "epsilon_greedy"
exploration_rate = 0.10  # 10% exploration
```

### Thompson Sampling
Bayesian approach that naturally balances exploration and exploitation based on observed reward distributions.

```toml
[routing]
policy = "thompson_sampling"
```

### Upper Confidence Bound (UCB)
Optimistic exploration bonus that favors under-sampled providers.

### Contextual Bandit
A per-arm LinUCB model over semantic prompt features (domain, difficulty, tool-need, sensitivity — see the laya sidecar below) that learns which model fits which kind of prompt. Cold-start spreads requests round-robin until rewards arrive; every reward updates the arm's ridge model with the features that drove the selection.

### Reward Function
Each provider arm accumulates reward from two real signals:

- **Request outcomes** — a successful provider response adds a positive reward; a failure adds zero (and the router fails over to the next provider).
- **User feedback** — star ratings (1–5) are scaled to 0–1, weighted by feedback type (quality and overall count at full weight, speed and cost less), and adjusted by a keyword sentiment scan of the comment ("excellent" pushes the reward up, "terrible" pulls it down). Feedback submitted to `/feedback` is applied to the bandit arms with per-model watermarks, so rewards are never double-counted.

> **Note:** the contextual bandit and `llm_classifier` consume features from the
> laya sidecar (below). Without it, the bandit falls back to a degraded
> prompt-length signal and the classifier falls back to a pure length
> heuristic — both work, but with much less discrimination.

---

## 🧬 Semantic Routing with the Laya Sidecar

Merlin can classify every prompt semantically before routing by talking to a
small sidecar service that wraps [laya](https://pypi.org/project/laya/) — a
non-autoregressive decision engine (~33 ms per prompt on GPU, 100+ languages).
The sidecar replaces Merlin's old length-based heuristics with real signal:

| Signal | Type | Drives |
|--------|------|--------|
| `domain` | choice (code, math, writing, factual, data, chitchat) | contextual bandit context vector |
| `difficulty` | score 0–1 | `llm_classifier` strong/weak split, bandit context |
| `is_sensitive` | probability | forces `llm_classifier` to the strong model ≥ 0.7 |
| `needs_tools` | probability | bandit context |
| `jailbreak` / `prompt_injection` | probability | pre-request guard (HTTP 400) |

### Quick Start

```bash
# 1. Start the sidecar (downloads laya checkpoints on first run)
pip install -r sidecar/requirements.txt
python sidecar/app.py --port 8765

# 2. Point Merlin at it
./target/release/merlin serve --config examples/laya-sidecar.toml
```

Everything is config-gated and **fail-open**: if the sidecar is down, slow, or
returns garbage, Merlin logs a warning and falls back to prompt-length
heuristics — routing never breaks because the classifier did. Prompt features
are cached per Merlin process (default 512 entries, 300 s TTL). Set
`guard_enabled = true` to block jailbreak/injection prompts with HTTP 400
before any provider is contacted. See `sidecar/README.md` for the HTTP API and
`examples/laya-sidecar.toml` for the full `[features]` configuration.

---

## 📡 API

### Chat Endpoint

```bash
curl -X POST http://localhost:7777/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt": "What is the capital of France?", "max_tokens": 100}'
```

```json
{
  "response": "The capital of France is Paris.",
  "provider": "openai",
  "model": "gpt-4-turbo",
  "session_id": "9f2c..."
}
```

The model is chosen per-prompt by the routing policy; if the selected
provider fails, Merlin automatically fails over to the next candidate. Pass
`session_id` to `/feedback` to rate the response and teach the router.

### Health & Metrics

```bash
curl http://localhost:7777/health
curl http://localhost:7777/metrics
```

---

## 📊 Observability

Merlin exports Prometheus-compatible metrics and Jaeger traces out of the box:

| Metric | Description |
|--------|-------------|
| `merlin_requests_total` | Total requests routed |
| `merlin_latency_ms` | p50/p95/p99 latency per provider |
| `merlin_cost_usd` | Estimated cost per provider |
| `merlin_reward_score` | Average reward per provider |

**Grafana dashboard included** in `dashboards/`.

---

## 🚀 Deployment

### Systemd Service (Production)

```bash
# Build and install
cargo build --release
sudo ./scripts/install-systemd.sh

# Manage
sudo systemctl enable --now merlin
sudo systemctl status merlin
journalctl -u merlin -f
```

Service features:
- 🔒 Non-root execution with restricted permissions
- 📊 Memory limits and CPU scheduling
- 🔄 Auto-restart on failure
- 📝 Native journald logging

### Configuration Files

| File | Purpose |
|------|---------|
| `/etc/merlin/merlin.toml` | Main configuration |
| `/etc/merlin/merlin.env` | Environment variables |
| `/etc/systemd/system/merlin.service` | Service definition |

---

## 🏗️ Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client    │────▶│   Merlin    │────▶│  Provider   │
│  (Your App) │◀────│   Router    │◀────│   (LLM)     │
└─────────────┘     └──────┬──────┘     └─────────────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌─────────┐  ┌─────────┐  ┌─────────┐
        │ Metrics │  │  Traces │  │  Cache  │
        │(Prom)   │  │(Jaeger) │  │ (Redis) │
        └─────────┘  └─────────┘  └─────────┘
```

---

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
merlin app policy --id myapp --locality local_only --max-rpm 60 \
    --max-spend-day 20 --sensitive-local-only
merlin app token issue --id myapp

# 3. Call with the token
curl -X POST http://localhost:7777/v1/chat/completions \
  -H "Authorization: Bearer mrl_live_…" \
  -H "Content-Type: application/json" \
  -d '{"model": "default", "messages": [{"role": "user", "content": "hello"}]}'
```

Policy enforcement order: authenticate (401) → route allowlist (403) → quotas (429) → sensitivity
(sidecar `is_sensitive`, falling back to a built-in local detector) → eligibility gate (403 with
self-explaining conflicts). When sensitivity cannot be determined the gate fails **closed** and
applies the strictest policy; routing-side classification stays fail-open as before. Aggregate
(non-streamed) responses include a `merlin_trace` object recording the decision.

Notes:
- Apps with a daily spend cap can only route to targets that declare
  `price_input`/`price_output` (fail-closed).
- Spend accounting charges aggregate responses; streamed (`stream: true`)
  responses are not accounted (v1 limitation).
- Denials are counted in `merlin_governance_denials_total{code}` and
  include an `error.conflicts` list explaining the denial.
- Governance denials use a string `error.code` (e.g. `"policy_denied"`);
  legacy error responses (upstream failures) use a numeric `code` — check
  the shape before reading it.
- See `examples/governance.toml` for a full config.

---

## 🗺️ Roadmap

- [x] Multi-provider routing
- [x] Epsilon-greedy & Thompson sampling
- [x] Prometheus metrics
- [x] Systemd service deployment
- [x] Automatic provider failover
- [ ] TensorRT-LLM backend for local GPU inference
- [ ] Dynamic model warm/cold pool management
- [ ] Distributed routing cluster (Raft consensus)
- [ ] Circuit breakers per provider

---

## 🤝 Contributing

Contributions welcome! Areas of interest:

- Additional routing algorithms (UCB, EXP3)
- New provider backends (Azure OpenAI, Bedrock, Vertex)
- Performance benchmarks and profiling
- Grafana dashboard improvements

---

<div align="center">

**© 2026 Andrew White · RegicideOS Project · GPL v3**

</div>
