# Systemd Service Setup - Quick Start Guide

## Overview

This guide will help you set up Merlin as a systemd service for production deployment. The systemd service provides automatic startup, monitoring, and logging integration with the system.

## Prerequisites

- Linux system with systemd (Ubuntu 20.04+, CentOS 8+, Debian 10+)
- Rust 1.75+ and Cargo installed
- Merlin project repository access
- sudo privileges for service installation

## Installation

### 1. Build Merlin

```bash
# Navigate to project root
cd /Users/a/code/merlin

# Build release version
cargo build --release

# Verify binary exists
ls -la target/release/merlin
```

### 2. Run Installation Script

```bash
# Execute the systemd installation script
sudo ./scripts/install-systemd.sh

# Script will:
# - Create necessary directories
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

# Test HTTP endpoint (if in HTTP mode)
curl http://localhost:8080/health

# Test CLI functionality
merlin --help
```

## Configuration

### Environment Variables

Create `/etc/merlin/merlin.env`:

```bash
# Service mode: HttpServer, CliDaemon, or Hybrid
MERLIN_MODE=Hybrid

# HTTP server settings
MERLIN_HTTP_PORT=8080
MERLIN_BIND_ADDRESS=0.0.0.0

# Logging
RUST_LOG=info
MERLIN_LOG_LEVEL=Info

# Paths
MERLIN_DATA_DIR=/var/lib/merlin
MERLIN_CONFIG_DIR=/etc/merlin

# Performance
MERLIN_MAX_CONNECTIONS=1000
MERLIN_MEMORY_MAX=512M
```

### Configuration File

Create `/etc/merlin/merlin.toml`:

```toml
[service]
mode = "Hybrid"
http_port = 8080
bind_address = "0.0.0.0"
log_level = "Info"
data_directory = "/var/lib/merlin"
config_directory = "/etc/merlin"
enable_metrics = true
max_connections = 1000
request_timeout_seconds = 30

[security]
dynamic_user = true
no_new_privileges = true
private_devices = true
protect_system = "strict"
read_write_paths = ["/var/lib/merlin"]
read_only_paths = ["/etc/merlin"]

[resources]
memory_max = "512M"
memory_high = "256M"
limit_nofile = 65536
limit_nproc = 4096
timeout_start_sec = 30
timeout_stop_sec = 10
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

# Reload configuration (if supported)
sudo systemctl reload merlin

# Edit service configuration
sudo systemctl edit merlin

# Show service dependencies
systemctl list-dependencies merlin
```

## Troubleshooting

### Service Fails to Start

```bash
# Check service status for errors
systemctl status merlin

# View detailed error logs
journalctl -u merlin -n 50

# Check if port is available
ss -tlnp | grep :8080

# Check file permissions
ls -la /var/lib/merlin/
ls -la /etc/merlin/
```

### Permission Issues

```bash
# Fix directory permissions
sudo chown -R merlin:merlin /var/lib/merlin
sudo chown -R root:root /etc/merlin
sudo chmod 755 /var/lib/merlin
sudo chmod 755 /etc/merlin
```

### Configuration Validation

```bash
# Test configuration syntax
cargo run -- validate-config /etc/merlin/merlin.toml

# Check environment variables
sudo systemctl show merlin | grep Environment

# Test configuration loading
sudo systemctl restart merlin && journalctl -u merlin -n 20
```

## Service Modes

### HTTP Server Mode
Run Merlin as a pure HTTP API server:

```bash
# Set mode in environment file
echo "MERLIN_MODE=HttpServer" | sudo tee -a /etc/merlin/merlin.env

# Restart service
sudo systemctl restart merlin

# Test API endpoint
curl http://localhost:8080/api/v1/features
```

### CLI Daemon Mode
Run Merlin as a CLI command processor:

```bash
# Set mode in environment file
echo "MERLIN_MODE=CliDaemon" | sudo tee -a /etc/merlin/merlin.env

# Restart service
sudo systemctl restart merlin

# Service will process CLI commands from queue
```

### Hybrid Mode (Default)
Support both HTTP API and CLI operations:

```bash
# Default configuration supports both modes
# HTTP server runs on configured port
# CLI commands can be executed via merlin CLI
```

## Monitoring

### Service Health

```bash
# Check if service is running
systemctl is-active merlin

# Check resource usage
systemctl show merlin | grep -E "(MemoryCurrent|CPUUsage)"

# Check process information
ps aux | grep merlin
```

### Log Analysis

```bash
# Monitor service logs in real-time
journalctl -u merlin -f -o cat

# Analyze service performance
journalctl -u merlin --since "1 hour ago" | grep -i "error\|warn"

# Check startup sequence
journalctl -u merlin -b -n 30
```

### Metrics

If metrics are enabled:

```bash
# Access metrics endpoint
curl http://localhost:8080/metrics

# Monitor with Prometheus
# Configure Prometheus to scrape http://localhost:8080/metrics
```

## Backup and Recovery

### Configuration Backup

```bash
# Backup configuration directory
sudo tar -czf merlin-config-$(date +%Y%m%d).tar.gz /etc/merlin/

# Backup data directory
sudo tar -czf merlin-data-$(date +%Y%m%d).tar.gz /var/lib/merlin/
```

### Service Recovery

```bash
# Stop service if running
sudo systemctl stop merlin

# Restore configuration
sudo tar -xzf merlin-config-YYYYMMDD.tar.gz -C /

# Restore data
sudo tar -xzf merlin-data-YYYYMMDD.tar.gz -C /

# Fix permissions
sudo chown -R merlin:merlin /var/lib/merlin
sudo chown -R root:root /etc/merlin

# Start service
sudo systemctl start merlin
```

## Security Considerations

### File Permissions

```bash
# Verify secure permissions
ls -la /etc/merlin/  # Should be root:root 755
ls -la /var/lib/merlin/  # Should be merlin:merlin 755
ls -la /var/log/merlin/  # Should be merlin:merlin 755
```

### Network Security

```bash
# Check listening ports
ss -tlnp | grep merlin

# Configure firewall if needed
sudo ufw allow 8080/tcp  # If HTTP server exposed
```

### Service Hardening

```bash
# Check service security settings
systemctl cat merlin | grep -E "(DynamicUser|ProtectSystem|NoNewPrivileges)"

# Verify no root privileges
ps aux | grep merlin  # Should run as merlin or dynamic user
```

## Performance Tuning

### Resource Limits

Edit `/etc/systemd/system/merlin.service.d/override.conf`:

```ini
[Service]
MemoryMax=1G
MemoryHigh=512M
LimitNOFILE=131072
```

### Connection Tuning

```bash
# Increase maximum connections
echo "MERLIN_MAX_CONNECTIONS=2000" | sudo tee -a /etc/merlin/merlin.env

# Adjust thread pool
echo "MERLIN_TOKIO_THREADS=8" | sudo tee -a /etc/merlin/merlin.env
```

### Log Management

```bash
# Configure log rotation (if using file logging)
sudo nano /etc/logrotate.d/merlin

# Or use journald settings
sudo journalctl --vacuum-size=100M
sudo journalctl --vacuum-time=30days
```

## Uninstallation

### Remove Service

```bash
# Stop and disable service
sudo systemctl stop merlin
sudo systemctl disable merlin

# Remove service files
sudo rm /etc/systemd/system/merlin.service
sudo rm -rf /etc/systemd/system/merlin.service.d/

# Reload systemd
sudo systemctl daemon-reload

# Remove data and config (optional)
sudo rm -rf /var/lib/merlin
sudo rm -rf /etc/merlin
sudo rm -rf /var/log/merlin
```

---
**Quick Start Complete**: Merlin is now running as a systemd service with proper monitoring, logging, and security configuration.