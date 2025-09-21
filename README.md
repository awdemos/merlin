# 🧙‍♂️ Merlin: The AI Routing Wizard  
*Intelligent multi-provider LLM routing intended for RegicideOS*  

## Overview  
Merlin is a AI router that intelligently selects optimal language models based on real-time performance metrics. Built for developers who demand reliability and efficiency in their AI workflows.

## Features  
✨ **Multi-Provider Support**: OpenAI, Anthropic, Gemini, local GGUF  
🎯 **Smart Routing**: Epsilon-greedy & Thompson sampling algorithms  
⚡️ **Real-Time Metrics**: Latency, cost, and quality tracking  
🔮 **Quality Judging**: On-device GPT-2 reward modeling  
📊 **Observability**: Tracing, Prometheus metrics, Grafana dashboards  

## Quick Start  
```bash
# Clone the repository
git clone https://github.com/regicideos/merlin.git
cd merlin

# Build and run
cargo build --release
./target/release/merlin serve --port 7777 --config ./merlin.toml
```

## Configuration Example  
```toml
[providers]
openai = { api_key = "sk-...", model = "gpt-4-turbo" }
anthropic = { api_key = "sk-ant-", model = "claude-3-opus" }

[routing]
policy = "thompson_sampling"
exploration_rate = 0.15

[telemetry]
prometheus_port = 9090
jaeger_endpoint = "http://localhost:14268/api/traces"
```

## API Usage

Once the server is running, interact with Merlin through these HTTP endpoints:

### Chat Endpoint
```bash
curl -X POST http://localhost:7777/chat \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "What is the capital of France?",
    "max_tokens": 100
  }'
```

Response:
```json
{
  "response": "The capital of France is Paris.",
  "provider": "openai"
}
```

### Health Check
```bash
curl http://localhost:7777/health
```

Response:
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

### Metrics
```bash
curl http://localhost:7777/metrics
```

Response:
```json
{
  "requests_total": 42,
  "avg_latency_ms": 250.5,
  "providers": {
    "openai": { "requests": 25, "avg_latency": 200 },
    "anthropic": { "requests": 17, "avg_latency": 320 }
  }
}
```

## Performance Dashboard  
![Merlin Metrics](https://via.placeholder.com/600x300?text=Merlin+Performance+Dashboard)

## Contributing  
We welcome contributions! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License  
**GPL v3 License**  
Copyright © 2025 RegicideOS Project  

## Systemd Service Deployment

For production deployment, Merlin includes a complete systemd service configuration:

### Quick Install

```bash
# Build the release binary
cargo build --release

# Install as systemd service
sudo ./scripts/install-systemd.sh

# Verify service is running
systemctl status merlin
journalctl -u merlin -f
```

### Service Features

- 🔒 **Security Hardened**: Non-root execution with restricted permissions
- 📊 **Resource Managed**: Memory limits and CPU scheduling
- 🔄 **Auto-restart**: Automatic recovery on failures
- 📝 **Integrated Logging**: Native systemd journald support
- 🔧 **Redis Integration**: Automatic dependency management
- 📈 **Monitoring**: Health checks and metrics endpoints

### Service Management

```bash
# Start/stop service
sudo systemctl start merlin
sudo systemctl stop merlin

# Enable on boot
sudo systemctl enable merlin

# View logs
journalctl -u merlin -f

# Check health
curl http://localhost:4242/health
```

### Configuration

Service configuration is managed through:
- **Environment**: `/etc/merlin/merlin.env`
- **Settings**: `/etc/merlin/merlin.conf`
- **Systemd**: `/etc/systemd/system/merlin.service`

For detailed documentation, see `docs/systemd/README.md`.

---

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.  

This program is distributed in the hope that it will be useful,  
but WITHOUT ANY WARRANTY; without even the implied warranty of  
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the  
GNU General Public License for more details.  

You should have received a copy of the GNU General Public License  
along with this program. If not, see <https://www.gnu.org/licenses/>.
