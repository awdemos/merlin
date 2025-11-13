# Merlin Model Router Documentation

## Overview

Merlin is an intelligent multi-model AI infrastructure platform that helps you predictively determine which LLM is best-suited to respond to each input in your application, accelerating development, improving accuracy, and lowering costs.

## Key Features

- **Intelligent Model Routing**: Automatically select the optimal LLM based on your input and requirements
- **Cost & Latency Optimization**: Choose between speed, cost, or quality optimization
- **Fallback & Reliability**: Built-in timeout handling and fallback models
- **Function Calling**: Support for tool use and function execution
- **Structured Outputs**: Generate structured JSON responses reliably
- **Session Tracking**: Track routing decisions for feedback and analytics
- **OpenAI Compatibility**: Drop-in replacement for existing OpenAI integrations
- **Dynamic Configuration**: TOML-based model and provider configuration

## Quick Start

### Installation

```bash
# Build from source
cargo build --release

# Start server
./target/release/merlin serve --port 8080
```

### Basic Usage

```bash
# Health check
curl http://localhost:8080/health

# List available models
curl http://localhost:8080/v1/models

# Intelligent model selection
curl -X POST http://localhost:8080/modelSelect \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{"role": "user", "content": "Explain quantum computing"}],
    "models": ["gpt-4", "claude-3-opus", "gemini-pro"]
  }'

# OpenAI-compatible chat
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "auto",  # Let Merlin choose
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

## API Reference

### Model Selection

#### POST /modelSelect

Intelligently selects the best model for your input.

**Request Body:**
```json
{
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Your question here"}
  ],
  "models": ["model1", "model2", "model3"],
  "tradeoff": "cost",           // Optional: "cost" | "latency" | "quality"
  "timeout": 5,                  // Optional: seconds to wait for recommendation
  "default_model": "gpt-3.5-turbo", // Optional: fallback model
  "session_id": "optional-session-id" // Optional: for tracking
}
```

**Response:**
```json
{
  "recommended_model": "gpt-4",
  "confidence": 0.85,
  "reasoning": "Strong match for Technical domain; high complexity task",
  "alternatives": [
    {
      "model": "claude-3-opus",
      "confidence": 0.78,
      "estimated_cost": 0.015,
      "estimated_latency_ms": 2000
    }
  ],
  "estimated_cost": 0.03,
  "estimated_latency_ms": 2500,
  "session_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

### OpenAI-Compatible Endpoints

#### GET /v1/models

Lists all available models with their capabilities.

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "gpt-4-turbo-preview",
      "object": "model",
      "created": 1640995200,
      "owned_by": "openai"
    }
  ]
}
```

#### POST /v1/chat/completions

OpenAI-compatible chat completions with intelligent routing.

**Request Body:**
```json
{
  "model": "auto",  // Let Merlin choose, or specify model
  "messages": [
    {"role": "user", "content": "Your message"}
  ],
  "max_tokens": 1000,
  "temperature": 0.7,
  "tradeoff": "cost",        // Optional: optimization target
  "timeout": 5,               // Optional: timeout in seconds
  "default_model": "gpt-3.5-turbo", // Optional: fallback
  "tools": [                  // Optional: function calling
    {
      "type": "function",
      "function": {
        "name": "calculate",
        "description": "Perform calculations",
        "parameters": {
          "type": "object",
          "properties": {
            "expression": {"type": "string"}
          }
        }
      }
    }
  ],
  "response_format": {          // Optional: structured outputs
    "type": "json_object",
    "schema": {
      "type": "object",
      "properties": {
        "result": {"type": "string"}
      }
    }
  }
}
```

**Response:**
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Response here"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 56,
    "completion_tokens": 31,
    "total_tokens": 87
  },
  "session_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

### Feedback & Personalization

#### POST /feedback

Submit feedback on routing decisions to improve future recommendations.

**Request Body:**
```json
{
  "session_id": "123e4567-e89b-12d3-a456-426614174000",
  "provider": "openai",
  "model": "gpt-4",
  "rating": 1,              // 0 (bad) to 1 (good)
  "metrics": {
    "accuracy": 0.95,
    "relevance": 0.88,
    "helpfulness": 0.92
  },
  "feedback_type": "satisfaction" // Optional: "satisfaction" | "preference"
}
```

#### POST /preferences

Create or update user preferences for personalized routing.

**Request Body:**
```json
{
  "user_id": "user123",
  "optimize_for": "quality",      // "quality" | "speed" | "cost" | "balanced"
  "preferred_models": ["gpt-4", "claude-3-opus"],
  "avoid_models": ["gpt-3.5-turbo"],
  "custom_weights": {
    "gpt-4": 1.2,
    "claude-3-opus": 0.8
  },
  "domain_preferences": {
    "technical": 1.5,
    "creative": 1.0
  }
}
```

## Configuration

### Model Capabilities (capabilities.toml)

Define model capabilities and characteristics for intelligent routing.

```toml
"gpt-4-turbo-preview" = { 
    provider = "openai",
    model = "gpt-4-turbo-preview",
    cost_per_1k_tokens = 0.01,
    avg_latency_ms = 2500,
    max_tokens = 4096,
    context_window = 128000,
    strengths = ["analytical", "technical", "mathematical"],
    supports_streaming = true,
    supports_function_calling = true,
    supports_vision = true,
    supports_tools = true,
    
    [quality_scores]
    overall = 0.92
    creativity = 0.85
    reasoning = 0.9
    code = 0.9
    analytical = 0.91
}
```

### Server Configuration (merlin.toml)

Configure providers, routing, and server settings.

```toml
[server]
host = "0.0.0.0"
port = 8080

[routing]
default_tradeoff = "quality"
capabilities_file = "capabilities.toml"
timeout_seconds = 5

[providers.openai]
enabled = true
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"
models = ["gpt-4", "gpt-3.5-turbo"]

[providers.anthropic]
enabled = true
api_key = "${ANTHROPIC_API_KEY}"
base_url = "https://api.anthropic.com"
models = ["claude-3-opus", "claude-3-sonnet"]
```

## Advanced Features

### Cost & Latency Tradeoffs

Control optimization targets with the `tradeoff` parameter:

- **"quality"**: Maximize response quality (default)
- **"cost"**: Prefer cheaper models when quality loss is negligible
- **"latency"**: Prefer faster models for real-time applications

### Fallback & Reliability

Configure robust behavior with timeouts and fallbacks:

- **Timeout**: Maximum time to wait for routing recommendation
- **Default Model**: Fallback model if routing fails
- **Retry Logic**: Automatic retry with exponential backoff

### Function Calling

Pass tools to models that support function calling:

```json
{
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get current weather",
        "parameters": {
          "type": "object",
          "properties": {
            "location": {"type": "string"}
          }
        }
      }
    }
  ]
}
```

### Structured Outputs

Ensure consistent JSON responses:

```json
{
  "response_format": {
    "type": "json_object",
    "schema": {
      "type": "object",
      "properties": {
        "analysis": {"type": "string"},
        "confidence": {"type": "number"}
      }
    }
  }
}
```

### Session Tracking

Use session IDs for:

- **Feedback Collection**: Link feedback to specific routing decisions
- **Analytics**: Track routing performance over time
- **Personalization**: Learn user preferences
- **Debugging**: Trace routing decisions

## Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/merlin /usr/local/bin/
EXPOSE 8080
CMD ["merlin", "serve", "--port", "8080"]
```

```bash
docker build -t merlin .
docker run -p 8080:8080 -e OPENAI_API_KEY=your_key merlin
```

### Environment Variables

```bash
# Required for provider functionality
OPENAI_API_KEY=your_openai_key
ANTHROPIC_API_KEY=your_anthropic_key
GEMINI_API_KEY=your_gemini_key

# Optional configuration
REDIS_URL=redis://localhost:6379
MERLIN_CONFIG=/path/to/merlin.toml
RUST_LOG=info
```

## Examples

### Python Client

```python
import requests

base_url = "http://localhost:8080"

# Model selection
response = requests.post(f"{base_url}/modelSelect", json={
    "messages": [{"role": "user", "content": "Write a Python function"}],
    "models": ["gpt-4", "claude-3-opus"],
    "tradeoff": "cost"
})

result = response.json()
print(f"Recommended model: {result['recommended_model']}")
print(f"Session ID: {result['session_id']}")

# Submit feedback
requests.post(f"{base_url}/feedback", json={
    "session_id": result['session_id'],
    "rating": 1,
    "metrics": {"accuracy": 0.9}
})
```

### Node.js Client

```javascript
const axios = require('axios');

const base_url = 'http://localhost:8080';

// OpenAI-compatible chat
const response = await axios.post(`${base_url}/v1/chat/completions`, {
    model: 'auto',
    messages: [
        {role: 'user', content: 'Explain machine learning'}
    ],
    tradeoff: 'quality'
});

console.log('Selected model:', response.data.model);
console.log('Response:', response.data.choices[0].message.content);
```

## Error Handling

### Error Response Format

```json
{
  "error": {
    "message": "Detailed error description",
    "type": "error_type",
    "code": "error_code"
  }
}
```

### Common Error Codes

- **`invalid_request`**: Malformed request or invalid parameters
- **`authentication_error`**: Invalid or missing API keys
- **`rate_limit_error`**: Too many requests
- **`model_unavailable`**: Requested model is not available
- **`routing_error`**: Failed to determine optimal model
- **`timeout_error`**: Request timed out

## Monitoring & Metrics

### Health Check

```bash
curl http://localhost:8080/health
```

Returns server status and version information.

### Metrics Endpoint

```bash
curl http://localhost:8080/metrics
```

Returns routing statistics, performance metrics, and system health.

## Troubleshooting

### Common Issues

**Server won't start:**
- Check Redis is running: `redis-cli ping`
- Verify configuration files exist and are valid
- Check port availability

**Poor routing quality:**
- Ensure capabilities.toml is up to date
- Verify model costs and latencies are accurate
- Collect feedback to improve routing

**API key errors:**
- Set environment variables correctly
- Check API key validity with providers
- Verify provider configuration in merlin.toml

### Debug Mode

```bash
RUST_LOG=debug ./target/release/merlin serve --port 8080
```

Enable detailed logging for troubleshooting.

## Best Practices

### Performance Optimization

1. **Use appropriate tradeoffs**: Choose "cost" for high-volume applications
2. **Set realistic timeouts**: 5-10 seconds for most use cases
3. **Configure fallbacks**: Always specify a reliable fallback model
4. **Collect feedback**: Regular feedback improves routing accuracy

### Security

1. **Secure API keys**: Use environment variables, not config files
2. **Enable HTTPS**: Use reverse proxy for production
3. **Rate limiting**: Implement client-side rate limiting
4. **Audit logs**: Monitor routing decisions and access patterns

### Reliability

1. **Health checks**: Monitor /health endpoint
2. **Circuit breakers**: Disable failing providers temporarily
3. **Load balancing**: Distribute load across providers
4. **Graceful degradation**: Fallback to simpler models when needed

## Support

For issues, feature requests, or contributions:

- **Documentation**: Check this guide and API reference
- **Issues**: Report bugs with detailed reproduction steps
- **Discussions**: Ask questions and share use cases
- **Contributions**: Pull requests welcome for features and fixes