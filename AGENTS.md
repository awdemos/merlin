# Merlin Development Guide

## Commands

### Build and Test
```bash
# Development build
cargo build

# Release build  
cargo build --release

# Run tests
cargo test

# Run with clippy linting
cargo clippy

# Format code
cargo fmt
```

### Running the Application
```bash
# Start the server
./target/release/merlin serve --port 8080

# With configuration file
./target/release/merlin serve --config ./merlin.toml
```

### CI/CD with Dagger
```bash
# Run CI pipeline locally
cd ci && go run main.go
```

## Project Structure

- `src/lib.rs` - Core library with Router implementation
- `src/main.rs` - Binary entry point with CLI
- `src/server.rs` - HTTP server with health/chat/metrics endpoints
- `src/providers/` - LLM provider implementations (OpenAI, Ollama)
- `src/routing/` - Routing policies (Epsilon-Greedy, Thompson Sampling)
- `src/metrics/` - Redis-based metrics collection
- `tests/` - Integration tests
- `ci/` - Dagger Go SDK CI pipeline

## Dependencies

### External Services
- **Redis** - For metrics storage (required)
- **Ollama** - Local LLM inference (optional)
- **OpenAI API** - Cloud LLM provider (optional)

### Testing
- Unit tests use `mockall` for mocking
- Integration tests in `tests/` directory
- CI runs tests, linting, and formatting checks

## Configuration

Use `merlin.toml` to configure:
- Server settings (host, port)
- Provider settings (API keys, endpoints) 
- Routing policies and parameters
- Metrics and telemetry settings

## Docker

```bash
# Build image
docker build -t merlin:latest .

# Run container
docker run -p 8080:8080 merlin:latest
```
