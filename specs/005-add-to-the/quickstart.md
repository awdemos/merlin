# Quick Start Guide: API Reference Endpoints

This guide provides a quick introduction to using the new API endpoints for model selection, feedback submission, and user preference management in the Merlin AI Router.

## Prerequisites

- Rust 1.75+ installed
- Merlin AI Router project checked out
- Redis server running (for preference storage)
- API access key (for authentication)

## Getting Started

### 1. Start the Development Server

```bash
# Clone the repository (if not already done)
git clone https://github.com/your-org/merlin.git
cd merlin

# Checkout the feature branch
git checkout 005-add-to-the

# Start Redis (if not already running)
redis-server

# Build and run the development server
cargo run --bin merlin-api
```

The server will start on `http://localhost:8080`.

### 2. Authentication

All API requests require authentication using an API key:

```bash
# Set your API key
export API_KEY="your-api-key-here"

# Or include it in request headers
Authorization: Bearer your-api-key-here
```

## API Endpoints

### Model Selection

**POST** `/api/v1/modelSelect`

Submit a prompt for intelligent model selection and processing:

```bash
curl -X POST http://localhost:8080/api/v1/modelSelect \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "prompt": "Explain quantum computing in simple terms",
    "max_tokens": 500,
    "temperature": 0.7
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "request_id": "123e4567-e89b-12d3-a456-426614174000",
    "selected_model": "gpt-4",
    "response": "Quantum computing is a revolutionary approach...",
    "tokens_used": 150,
    "processing_time": 850,
    "confidence_score": 0.92,
    "metadata": {
      "server_version": "1.0.0",
      "processing_time_ms": 150,
      "request_timestamp": "2024-01-01T12:00:00Z",
      "response_timestamp": "2024-01-01T12:00:01Z",
      "rate_limit_remaining": 99
    }
  },
  "error": null,
  "metadata": {
    "server_version": "1.0.0",
    "processing_time_ms": 150,
    "request_timestamp": "2024-01-01T12:00:00Z",
    "response_timestamp": "2024-01-01T12:00:01Z",
    "rate_limit_remaining": 99
  },
  "request_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

### Feedback Submission

**POST** `/api/v1/feedback`

Submit feedback on model performance:

```bash
curl -X POST http://localhost:8080/api/v1/feedback \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "request_id": "123e4567-e89b-12d3-a456-426614174000",
    "model_name": "gpt-4",
    "rating": 5,
    "feedback_text": "The response was very helpful and accurate",
    "category": "Helpfulness"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "456e7890-e12b-34c5-a678-901234567890",
    "user_id": "user123",
    "request_id": "123e4567-e89b-12d3-a456-426614174000",
    "model_name": "gpt-4",
    "rating": 5,
    "feedback_text": "The response was very helpful and accurate",
    "category": "Helpfulness",
    "created_at": "2024-01-01T12:01:00Z"
  },
  "error": null,
  "metadata": {
    "server_version": "1.0.0",
    "processing_time_ms": 50,
    "request_timestamp": "2024-01-01T12:01:00Z",
    "response_timestamp": "2024-01-01T12:01:00Z",
    "rate_limit_remaining": 98
  },
  "request_id": "789e0123-f45a-67b8-c901-234567890123"
}
```

### User Preference Management

#### Create Preference

**POST** `/api/v1/preferences/userPreferenceCreate`

```bash
curl -X POST http://localhost:8080/api/v1/preferences/userPreferenceCreate \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "preference_key": "preferred_models",
    "preference_value": ["gpt-4", "claude-3"],
    "category": "ModelSelection"
  }'
```

#### Update Preference

**PUT** `/api/v1/preferences/userPreferenceUpdate/{preference_id}`

```bash
curl -X PUT http://localhost:8080/api/v1/preferences/userPreferenceUpdate/123e4567-e89b-12d3-a456-426614174000 \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -H "If-Match: 1" \
  -d '{
    "preference_value": ["gpt-4", "claude-3", "gemini-pro"]
  }'
```

#### Delete Preference

**DELETE** `/api/v1/preferences/userPreferenceDelete/{preference_id}`

```bash
curl -X DELETE http://localhost:8080/api/v1/preferences/userPreferenceDelete/123e4567-e89b-12d3-a456-426614174000 \
  -H "Authorization: Bearer $API_KEY"
```

## Common Use Cases

### 1. Basic Model Selection

```bash
# Simple prompt processing
curl -X POST http://localhost:8080/api/v1/modelSelect \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "prompt": "What is the capital of France?"
  }'
```

### 2. Model Selection with Preferences

```bash
# Advanced selection with user preferences
curl -X POST http://localhost:8080/api/v1/modelSelect \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "prompt": "Write a Python function to calculate fibonacci numbers",
    "max_tokens": 300,
    "temperature": 0.5,
    "model_preferences": {
      "preferred_models": ["gpt-4", "claude-3"],
      "excluded_models": ["gpt-3.5"],
      "max_cost": 0.02
    }
  }'
```

### 3. Submit Multiple Feedback Types

```bash
# Accuracy feedback
curl -X POST http://localhost:8080/api/v1/feedback \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "model_name": "gpt-4",
    "rating": 5,
    "category": "Accuracy"
  }'

# Performance feedback
curl -X POST http://localhost:8080/api/v1/feedback \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "model_name": "claude-3",
    "rating": 4,
    "feedback_text": "Response was good but a bit slow",
    "category": "Performance"
  }'
```

### 4. Manage User Preferences

```bash
# Create model preferences
curl -X POST http://localhost:8080/api/v1/preferences/userPreferenceCreate \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "preference_key": "preferred_models",
    "preference_value": ["gpt-4", "claude-3"],
    "category": "ModelSelection"
  }'

# Create formatting preferences
curl -X POST http://localhost:8080/api/v1/preferences/userPreferenceCreate \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "preference_key": "response_style",
    "preference_value": "concise",
    "category": "ResponseFormatting"
  }'
```

## Error Handling

The API uses standard HTTP status codes and provides detailed error information:

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid request parameters",
    "details": "Field 'user_id' is required",
    "field_errors": [
      {
        "field": "user_id",
        "message": "Field is required",
        "code": "REQUIRED_FIELD"
      }
    ]
  },
  "metadata": {
    "server_version": "1.0.0",
    "processing_time_ms": 5,
    "request_timestamp": "2024-01-01T12:00:00Z",
    "response_timestamp": "2024-01-01T12:00:00Z",
    "rate_limit_remaining": 99
  },
  "request_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

## Rate Limiting

The API implements rate limiting to ensure fair usage:

- **Default limit**: 100 requests per minute per user
- **Burst limit**: 10 requests per second
- **Response headers include remaining quota**

## Configuration

The API can be configured through environment variables and TOML configuration files:

```toml
[api]
host = "0.0.0.0"
port = 8080
rate_limit_requests = 100
rate_limit_window = 60

[redis]
url = "redis://localhost:6379"
pool_size = 10

[models]
default_max_tokens = 1000
default_temperature = 0.7
timeout = 30
```

## Testing

Run the test suite to verify API functionality:

```bash
# Run all tests
cargo test

# Run API-specific tests
cargo test api

# Run integration tests
cargo test --test integration
```

## Next Steps

1. **Review the full API documentation** at `/docs/api.html` when the server is running
2. **Explore the OpenAPI specification** in `specs/005-add-to-the/contracts/api.yaml`
3. **Check the data models** in `specs/005-add-to-the/data-model.md`
4. **Run the comprehensive test suite** to understand all API behaviors
5. **Integrate with your application** using the provided examples

## Support

For issues or questions:
- Check the [API documentation](http://localhost:8080/docs)
- Review the [OpenAPI specification](specs/005-add-to-the/contracts/api.yaml)
- Run the test suite for usage examples
- Contact the development team for support

This quick start guide covers the essential functionality of the new API endpoints. For more detailed information, refer to the full documentation and specifications.