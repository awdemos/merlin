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
A per-model linear model over prompt features (domain, task type, complexity, keywords) that learns which model fits which kind of prompt.

### Reward Function
Each provider arm accumulates reward from two real signals:

- **Request outcomes** — a successful provider response adds a positive reward; a failure adds zero (and the router fails over to the next provider).
- **User feedback** — star ratings (1–5) are scaled to 0–1, weighted by feedback type (quality and overall count at full weight, speed and cost less), and adjusted by a keyword sentiment scan of the comment ("excellent" pushes the reward up, "terrible" pulls it down). Feedback submitted to `/feedback` is applied to the bandit arms with per-model watermarks, so rewards are never double-counted.

> **Note:** prompt embeddings in the contextual bandit are currently a
> lightweight hash-based placeholder, not a trained embedding model. Routing
> still uses the structured prompt features (domain, task, complexity,
> keywords), but the embedding dimensions carry no semantic signal yet.

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
