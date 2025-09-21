# Merlin Systemd Service Configuration

This directory contains the systemd service configuration for running Merlin as a system daemon.

## Overview

The Merlin systemd service provides production-ready deployment with:
- **Service Management**: Automatic start/stop/restart capabilities
- **Security Hardening**: Non-root execution with restricted permissions
- **Logging Integration**: Native systemd journald logging
- **Resource Management**: Memory and CPU limits
- **Monitoring**: Health checks and metrics endpoints
- **Dependencies**: Proper Redis and network service ordering

## Quick Start

### 1. Build Merlin

```bash
# Navigate to project root
cd /path/to/merlin

# Build release version
cargo build --release

# Verify binary exists
ls -la target/release/merlin
```

### 2. Install Service

```bash
# Run the installation script
sudo ./scripts/install-systemd.sh

# The script will:
# - Create required directories
# - Install service file
# - Set up configuration
# - Enable and start the service
```

### 3. Verify Installation

```bash
# Check service status
systemctl status merlin

# View service logs
journalctl -u merlin -f

# Test HTTP endpoint
curl http://localhost:4242/health

# Test CLI functionality
merlin --help
```

## Files

### Systemd Service File
- **Location**: `systemd/merlin.service`
- **Purpose**: Systemd service configuration
- **Key Features**:
  - Dynamic user management
  - Security hardening
  - Resource limits
  - Redis dependency management

### Installation Script
- **Location**: `scripts/install-systemd.sh`
- **Purpose**: Automated service installation
- **Capabilities**:
  - Directory creation
  - File installation
  - Permission setup
  - Service activation

### Configuration Files

#### Environment Configuration
- **Location**: `systemd/merlin.env`
- **Purpose**: Environment variables for service runtime
- **Format**: KEY=VALUE pairs

#### Daemon Configuration
- **Location**: `systemd/merlin.conf`
- **Purpose**: TOML configuration for service behavior
- **Format**: Structured TOML with sections

### Test Files
- **Location**: `tests/systemd/`
- **Purpose**: TDD tests for validation
- **Coverage**: Service files, installation, startup, configuration

## Configuration

### Service Modes

The Merlin service supports three operational modes:

1. **Hybrid** (Default): Both HTTP API and CLI operations
2. **HttpServer**: HTTP API only
3. **CliDaemon**: CLI command processing only

### Environment Variables

Key environment variables in `merlin.env`:

```bash
# Service configuration
MERLIN_MODE=Hybrid
MERLIN_HTTP_PORT=4242
MERLIN_BIND_ADDRESS=0.0.0.0

# Logging
RUST_LOG=info
MERLIN_LOG_LEVEL=Info

# Paths
MERLIN_DATA_DIR=/var/lib/merlin
MERLIN_CONFIG_DIR=/etc/merlin

# Redis (required for A/B testing, preferences, metrics)
REDIS_URL=redis://localhost:6379
```

### TOML Configuration

Key sections in `merlin.conf`:

```toml
[service]
mode = "Hybrid"
http_port = 4242
enable_metrics = true

[routing]
default_provider = "openai"
enable_ab_testing = true

[redis]
enabled = true
host = "localhost"
port = 6379
```

## Service Management

### Basic Commands

```bash
# Start service
sudo systemctl start merlin

# Stop service
sudo systemctl stop merlin

# Restart service
sudo systemctl restart merlin

# Enable service on boot
sudo systemctl enable merlin

# Disable service on boot
sudo systemctl disable merlin

# Check service status
sudo systemctl status merlin
```

### Advanced Management

```bash
# View live logs
journalctl -u merlin -f

# View logs since last boot
journalctl -u merlin -b

# View error logs only
journalctl -u merlin -p err

# Reload configuration
sudo systemctl reload merlin

# Edit service configuration
sudo systemctl edit merlin

# Show service dependencies
systemctl list-dependencies merlin
```

## Security

### Security Features

The service includes comprehensive security hardening:

- **Dynamic User**: No static user management required
- **No Privilege Escalation**: `NoNewPrivileges=true`
- **Filesystem Protection**: `ProtectSystem=strict`
- **Network Restrictions**: Limited to required protocols
- **Capability Restrictions**: No Linux capabilities
- **System Call Filtering**: Allowed system calls only
- **Resource Limits**: Memory and process constraints

### File Permissions

```
/etc/merlin/           # Configuration (root:root 755)
├── merlin.conf        # Daemon config (root:root 644)
└── merlin.env         # Environment (root:root 600)

/var/lib/merlin/       # Data storage (dynamic user)
├── features.json      # Feature data
└── cache/             # Cache files

/var/log/merlin/       # Logs (managed by journald)
/run/merlin/           # Runtime files
```

## Monitoring

### Health Checks

```bash
# HTTP health endpoint
curl http://localhost:4242/health

# Service status
systemctl is-active merlin

# Process information
ps aux | grep merlin

# Resource usage
systemctl show merlin | grep -E "(MemoryCurrent|CPUUsage)"
```

### Metrics

The service exposes metrics at `http://localhost:9090/metrics` when enabled.

### Logging

```bash
# Real-time logs
journalctl -u merlin -f -o cat

# Performance analysis
journalctl -u merlin --since "1 hour ago" | grep -i "error\|warn"

# Startup sequence
journalctl -u merlin -b -n 30
```

## Troubleshooting

### Service Fails to Start

```bash
# Check service status
systemctl status merlin

# View detailed error logs
journalctl -u merlin -n 50

# Check port availability
ss -tlnp | grep :4242

# Check file permissions
ls -la /var/lib/merlin/
ls -la /etc/merlin/
```

### Redis Connection Issues

```bash
# Check Redis status
systemctl status redis

# Test Redis connection
redis-cli ping

# Check Redis logs
journalctl -u redis -f
```

### Configuration Issues

```bash
# Test configuration syntax
cargo run -- validate-config /etc/merlin/merlin.conf

# Check environment variables
sudo systemctl show merlin | grep Environment

# Test configuration loading
sudo systemctl restart merlin && journalctl -u merlin -n 20
```

### Permission Issues

```bash
# Fix directory permissions
sudo chown -R merlin:merlin /var/lib/merlin
sudo chown -R root:root /etc/merlin
sudo chmod 755 /var/lib/merlin
sudo chmod 755 /etc/merlin
```

## Integration

### With Existing CLI

The service integrates seamlessly with Merlin's existing CLI:

```bash
# All CLI commands work with the service
merlin feature create "New Feature" --description "Description"
merlin feature list
merlin feature update --id 1 --status "in_progress"
```

### With HTTP API

The service exposes Merlin's HTTP API on port 4242:

```bash
# Access API endpoints
curl http://localhost:4242/api/v1/features
curl http://localhost:4242/api/v1/health
curl http://localhost:4242/metrics
```

### With Redis

The service requires Redis for:
- A/B testing experiment storage
- User preference management
- Performance metrics collection
- Feedback storage and analysis

## Performance

### Resource Usage

- **Memory**: 512MB max, 256MB high
- **CPU**: Idle scheduling priority
- **Connections**: 1000 concurrent HTTP connections
- **Threads**: 4 Tokio worker threads

### Optimization

The service includes performance optimizations:
- Connection pooling for Redis
- Caching for frequently accessed data
- Efficient resource usage monitoring
- Graceful shutdown handling

## Development

### Testing

Run the comprehensive test suite:

```bash
# Run all tests
cargo test

# Run systemd-specific tests
cargo test systemd_tests

# Test installation script
./scripts/install-systemd.sh --validate
```

### Development Mode

For development without systemd:

```bash
# Run Merlin directly
cargo run -- serve

# With custom configuration
cargo run -- serve --config /path/to/config.toml

# With environment variables
RUST_LOG=debug cargo run -- serve
```

## Architecture

### Service Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   systemd      │    │   merlin        │    │   Redis         │
│   service      │◄──►│   daemon        │◄──►│   database      │
│   manager      │    │   (Rust)        │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   journald     │    │   HTTP API      │    │   Metrics &     │
│   logging      │    │   (port 4242)   │    │   Preferences   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Data Flow

1. **HTTP Requests** → Merlin API → Business Logic
2. **Configuration** → Environment variables + TOML files
3. **Storage** → Redis (runtime) + JSON files (persistence)
4. **Logging** → systemd journald + structured logging
5. **Monitoring** → Health endpoints + metrics collection

## Support

### Getting Help

- Check logs: `journalctl -u merlin -f`
- Validate config: `./scripts/install-systemd.sh --validate`
- Test connectivity: `curl http://localhost:4242/health`
- Review documentation: See `/docs/` and `/specs/` directories

### Contributing

When making changes to the systemd configuration:

1. Update tests in `tests/systemd/`
2. Validate changes with `./scripts/install-systemd.sh --validate`
3. Test on multiple Linux distributions
4. Update documentation accordingly

---

**Note**: This systemd service configuration is designed for production deployment. For development purposes, you can run Merlin directly with `cargo run -- serve`.