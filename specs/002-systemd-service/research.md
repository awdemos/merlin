# Phase 0: Systemd Service Research Findings

## Systemd Best Practices for Rust Services

### Decision: ExecutableStart with dynamic user management
**Rationale**: systemd's dynamic user creation provides better security isolation than static system users. For a Rust service like Merlin that needs file access and network capabilities, dynamic users with restrictive permissions are ideal.

**Key findings**:
- `DynamicUser=yes` creates ephemeral users with no shell access
- `ReadWritePaths=` directive replaces traditional file permissions
- `NoNewPrivileges=true` prevents privilege escalation
- Rust services benefit from `ProtectSystem=strict` for filesystem protection

**Alternatives considered**:
- **Static system user**: Rejected due to manual user management overhead
- **Root execution**: Rejected due to security risks and constitutional violations
- **Docker container**: Rejected as overkill for simple daemon functionality

## CLI Tool as Systemd Service Patterns

### Decision: Hybrid service supporting both CLI and HTTP modes
**Rationale**: Merlin already has both CLI commands and HTTP server functionality. The systemd service should support both operational modes through environment variables and command-line arguments.

**Service configuration approach**:
```ini
[Service]
# Default to HTTP server mode
ExecStart=/usr/local/bin/merlin serve
# Allow CLI mode override
EnvironmentFile=-/etc/merlin/merlin.env
```

**Alternatives considered**:
- **Separate services**: Rejected due to complexity and resource duplication
- **CLI-only daemon**: Rejected as it would break existing HTTP API functionality
- **HTTP-only daemon**: Rejected as it would disable CLI capabilities

## Environment Variable Management in Systemd

### Decision: Layered configuration approach
**Rationale**: Systemd services need flexible configuration that works across different environments while maintaining security. A layered approach provides maximum flexibility.

**Configuration layers**:
1. **Service defaults**: Hardcoded fallbacks in the application
2. **EnvironmentFile**: `/etc/merlin/merlin.env` for system-wide settings
3. **Drop-in files**: `/etc/systemd/system/merlin.service.d/` for overrides
4. **Runtime overrides**: `systemctl edit merlin` for temporary changes

**Security considerations**:
- `EnvironmentFile=-` (with dash) makes file optional
- `PrivateTmp=true` isolates temporary files
- `ProtectHome=true` prevents home directory access

**Alternatives considered**:
- **Configuration files only**: Rejected due to lack of runtime flexibility
- **Environment variables only**: Rejected due to security and organization concerns
- **Command-line arguments only**: Rejected due to complexity and lack of persistence

## Security Hardening for System Daemons

### Decision: Comprehensive systemd security directives
**Rationale**: Merlin processes potentially sensitive routing data and LLM API keys, requiring strong security isolation.

**Security measures**:
```ini
[Service]
# User isolation
DynamicUser=yes
NoNewPrivileges=true
PrivateDevices=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/merlin /var/log/merlin

# Network isolation
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
PrivateTmp=true

# Resource limits
LimitNOFILE=65536
MemoryMax=512M
```

**Alternatives considered**:
- **Minimal security**: Rejected due to inadequate protection for AI service
- **Container isolation**: Rejected due to overhead and complexity
- **SELinux/AppArmor**: Rejected due to distribution compatibility issues

## Service Lifecycle Management

### Decision: Standard systemd lifecycle with health checks
**Rationale**: Merlin needs proper startup, shutdown, and health monitoring to ensure reliable operation as a system service.

**Lifecycle configuration**:
```ini
[Service]
Type=notify
NotifyAccess=all
TimeoutStartSec=30
TimeoutStopSec=10
Restart=on-failure
RestartSec=5
```

**Health monitoring approach**:
- HTTP endpoint health checks for server mode
- Process monitoring for CLI operations
- Log-based health assessment
- systemd watchdog integration

**Alternatives considered**:
- **Simple process monitoring**: Rejected due to insufficient health assessment
- **External monitoring**: Rejected as it adds external dependencies
- **Custom health system**: Rejected due to reinventing systemd capabilities

## Installation and Deployment Strategy

### Decision: Idempotent shell script with validation
**Rationale**: Systemd services need proper installation with user management, file placement, and service activation. An idempotent script ensures reliable deployment.

**Installation workflow**:
1. Pre-flight checks (systemd availability, permissions)
2. User and group creation (if not using dynamic users)
3. Directory structure creation with proper permissions
4. Configuration file deployment
5. Service file installation
6. Service enablement and activation
7. Validation testing

**Alternatives considered**:
- **Manual installation**: Rejected due to error-prone and inconsistent setup
- **Package manager integration**: Rejected due to distribution complexity
- **Ansible/Chef**: Rejected due to additional dependencies

## Logging Integration

### Decision: Native systemd logging with journald
**Rationale**: Systemd's journald provides structured logging, rotation, and integration with system monitoring tools. Rust's tracing ecosystem integrates well with journald.

**Logging configuration**:
```ini
[Service]
StandardOutput=journal
StandardError=journal
SyslogIdentifier=merlin
```

**Application integration**:
- Use `tracing` crate with `tracing-journald` subscriber
- Structured logging with service context
- Log level filtering via environment variables
- Correlation IDs for request tracking

**Alternatives considered**:
- **File-based logging**: Rejected due to rotation and management overhead
- **Syslog integration**: Rejected as journald provides better integration
- **Custom logging**: Rejected due to reinventing existing solutions

## Filesystem and Permissions

### Decision: Restricted filesystem access with specific paths
**Rationale**: Merlin needs to access feature data and configuration while maintaining security isolation.

**Access strategy**:
```ini
[Service]
# Strict protection by default
ProtectSystem=strict
ProtectHome=true

# Specific read/write paths
ReadWritePaths=/var/lib/merlin
ReadOnlyPaths=/etc/merlin
```

**Directory structure**:
```
/etc/merlin/          # Configuration files (read-only)
/var/lib/merlin/      # Data storage (read-write)
/var/log/merlin/      # Log files (managed by journald)
/run/merlin/          # Runtime files (sockets, pids)
```

**Alternatives considered**:
- **Home directory storage**: Rejected due to security and multi-user concerns
- **Application directory**: Rejected due to packaging standards
- **Database storage**: Rejected due to overkill for feature data

## Performance and Resource Management

### Decision: Resource limits with monitoring integration
**Rationale**: As an AI routing service, Merlin needs predictable resource usage and monitoring capabilities.

**Resource configuration**:
```ini
[Service]
# Memory limits
MemoryMax=512M
MemoryHigh=256M

# File limits
LimitNOFILE=65536
LimitNPROC=4096

# CPU priority
Nice=10
CPUSchedulingPolicy=idle
```

**Monitoring integration**:
- systemd resource usage tracking
- Prometheus metrics endpoint integration
- Memory usage alerts
- Connection pool monitoring

**Alternatives considered**:
- **Unlimited resources**: Rejected due to system stability concerns
- **External resource management**: Rejected due to complexity
- **Application-level limits**: Rejected as systemd provides better integration

---
**Research Complete**: All technical decisions documented and justified
**Ready for**: Phase 1 design and task generation