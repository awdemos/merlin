# Systemd Service User and Permission Requirements

## Service User Configuration

### Dynamic User (Recommended)
Based on systemd best practices research, Merlin will use systemd's dynamic user functionality:

```ini
[Service]
DynamicUser=yes
```

**Benefits:**
- No manual user management required
- Automatic cleanup on service removal
- Enhanced security isolation
- No password or shell access

### Alternative: Static System User
If dynamic users are not available:

```bash
# Create system user and group
sudo groupadd --system merlin
sudo useradd --system --no-create-home --shell /usr/sbin/nologin -g merlin merlin
```

## Filesystem Permissions

### Directory Structure and Permissions

```
/etc/merlin/           # Configuration files
├── merlin.toml        # Main configuration (root:root 644)
└── merlin.env         # Environment variables (root:root 600)

/var/lib/merlin/       # Data storage (merlin:merlin 750)
├── features.json      # Feature data (merlin:merlin 644)
└── cache/             # Cache files (merlin:merlin 750)

/var/log/merlin/       # Log files (managed by journald)
/run/merlin/           # Runtime files (sockets, pids)
```

### Systemd Permission Directives

```ini
[Service]
# Filesystem protection
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/merlin
ReadOnlyPaths=/etc/merlin

# Device isolation
PrivateDevices=true
DevicePolicy=closed

# Network restrictions
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
PrivateTmp=true

# No privilege escalation
NoNewPrivileges=true
```

## Resource Limits

### Memory Limits
```ini
[Service]
MemoryMax=512M
MemoryHigh=256M
```

### File and Process Limits
```ini
[Service]
LimitNOFILE=65536
LimitNPROC=4096
```

### CPU Scheduling
```ini
[Service]
Nice=10
CPUSchedulingPolicy=idle
```

## Security Requirements

### Capability Dropping
```ini
[Service]
CapabilityBoundingSet=
AmbientCapabilities=
```

### System Call Filtering
```ini
[Service]
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM
```

### Temporary File Isolation
```ini
[Service]
PrivateTmp=true
RemoveIPC=true
```

## Access Control

### Network Access
- HTTP server binds to specific interface (default: 127.0.0.1)
- Port configurable via environment (default: 8080)
- Optional external access with firewall rules

### File Access
- Read access to configuration: `/etc/merlin/`
- Read/write access to data: `/var/lib/merlin/`
- No access to user home directories
- No access to system files

### Process Access
- Limited to merlin user context
- No root privileges
- Restricted system call access

## Installation Requirements

### System Requirements
- Linux with systemd (Ubuntu 20.04+, CentOS 8+, Debian 10+)
- Rust 1.75+ for building from source
- sudo privileges for service installation

### Runtime Dependencies
- None (self-contained Rust binary)

### Optional Dependencies
- Redis for caching (if configured)
- Prometheus for metrics (if enabled)

## Service Dependencies

### Required
```ini
[Unit]
After=network.target
Wants=network.target
```

### Optional (Redis)
```ini
[Unit]
After=redis.service
Wants=redis.service
```

## Logging Requirements

### Integration
```ini
[Service]
StandardOutput=journal
StandardError=journal
SyslogIdentifier=merlin
```

### Log Rotation
- Managed by systemd journald
- Optional external log rotation for file-based logging

## Validation Requirements

### Service Validation
- Verify service runs as non-root user
- Check resource limits are enforced
- Validate network restrictions work
- Test file access permissions

### Configuration Validation
- Check configuration file permissions
- Validate environment variable loading
- Test configuration reload capability

### Security Validation
- Verify no privilege escalation possible
- Test capability restrictions
- Validate filesystem isolation

## Summary

This configuration provides:
- ✅ Non-root execution
- ✅ Restricted filesystem access
- ✅ Resource limits
- ✅ Network isolation
- ✅ Secure temporary file handling
- ✅ Comprehensive systemd integration
- ✅ Defense-in-depth security approach

The requirements align with systemd best practices for Rust services and provide a secure foundation for running Merlin as a system daemon.