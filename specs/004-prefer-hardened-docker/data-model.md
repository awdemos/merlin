# Data Model: Hardened Docker Deployment

## Core Entities

### DockerContainerConfig
Represents the configuration for hardened Docker container deployment.

**Fields**:
- `image_name: String` - Container image name (e.g., "merlin:latest")
- `base_image: String` - Base image type ("distroless", "alpine", "wolfi")
- `user_name: String` - Non-root username ("merlin")
- `user_id: Integer` - Numeric user ID (default: 1000)
- `group_id: Integer` - Numeric group ID (default: 1000)

### SecurityProfile
Defines security constraints and hardening settings for the container.

**Fields**:
- `read_only_filesystem: Boolean` - Enable read-only root filesystem
- `no_new_privileges: Boolean` - Prevent privilege escalation
- `drop_capabilities: Array<String>` - Linux capabilities to drop
- `security_opt: Array<String>` - Security options (e.g., "no-new-privileges")
- `tmpfs_mounts: Array<TmpfsMount>` - Temporary filesystem mounts

### ResourceLimits
Defines resource constraints for container execution.

**Fields**:
- `memory_limit: String` - Memory limit (e.g., "512m", "1g")
- `memory_high: String` - Memory high watermark
- `memory_swap: String` - Memory + swap limit
- `cpu_limit: Float` - CPU shares (0.0-2.0)
- `pids_limit: Integer` - Process ID limit
- `open_files_limit: Integer` - Open file descriptor limit

### TmpfsMount
Represents a temporary filesystem mount point.

**Fields**:
- `path: String` - Mount path within container
- `size: String` - Size limit (e.g., "100m")
- `permissions: String` - Mount permissions (e.g., "rw,noexec,nosuid")

### HealthMonitor
Defines container health monitoring configuration.

**Fields**:
- `endpoint: String` - Health check endpoint path
- `interval_seconds: Integer` - Check interval
- `timeout_seconds: Integer` - Check timeout
- `retries: Integer` - Number of retries before unhealthy
- `start_period_seconds: Integer` - Startup grace period

### SecurityScanConfig
Configuration for security scanning and compliance.

**Fields**:
- `enabled: Boolean` - Enable security scanning
- `severity_threshold: String` - Minimum severity to fail ("critical", "high", "medium")
- `ignore_vulnerabilities: Array<String>` - CVE IDs to ignore
- `compliance_benchmark: String` - Compliance standard ("cis-docker")

### DeploymentEnvironment
Environment-specific deployment configuration.

**Fields**:
- `name: String` - Environment name ("development", "staging", "production")
- `resource_multiplier: Float` - Resource scaling factor
- `security_level: String` - Security strictness level ("strict", "moderate", "permissive")
- `monitoring_enabled: Boolean` - Enable enhanced monitoring
- `debug_enabled: Boolean` - Enable debug features

## Relationships

```
DockerContainerConfig
├── SecurityProfile (1:1)
├── ResourceLimits (1:1)
├── HealthMonitor (1:1)
├── SecurityScanConfig (1:1)
└── DeploymentEnvironment[] (1:N)
```

## State Transitions

### Container Lifecycle States
1. **BUILDING** → Creating container image
2. **SCANNING** → Security scanning in progress
3. **READY** → Container ready for deployment
4. **RUNNING** → Container actively serving requests
5. **UNHEALTHY** → Health checks failing
6. **STOPPED** → Container stopped
7. **ERROR** → Container in error state

### Security Validation States
1. **PENDING** → Awaiting security scan
2. **SCANNING** → Scan in progress
3. **COMPLIANT** → All checks passed
4. **VIOLATION** → Security issues found
5. **EXEMPTED** → Violations explicitly exempted

## Validation Rules

### Container Configuration Validation
- `image_name` must follow Docker naming conventions
- `user_id` and `group_id` must be > 0 and < 65535
- `memory_limit` must be valid format with reasonable bounds
- `cpu_limit` must be between 0.1 and 4.0
- `base_image` must be one of supported types

### Security Profile Validation
- `read_only_filesystem` requires `tmpfs_mounts` for writable paths
- `drop_capabilities` cannot include essential capabilities
- `security_opt` must be valid Docker security options

### Resource Limits Validation
- `memory_high` must be <= `memory_limit`
- `memory_swap` must be >= `memory_limit`
- `pids_limit` must be >= 100 and <= 10000
- `open_files_limit` must be >= 1024

### Health Monitor Validation
- `interval_seconds` must be >= 5 and <= 300
- `timeout_seconds` must be <= `interval_seconds`
- `retries` must be >= 1 and <= 10

## Default Values

### Default Security Profile
```rust
SecurityProfile {
    read_only_filesystem: true,
    no_new_privileges: true,
    drop_capabilities: vec!["ALL"],
    security_opt: vec!["no-new-privileges"],
    tmpfs_mounts: vec![
        TmpfsMount {
            path: "/tmp".to_string(),
            size: "100m".to_string(),
            permissions: "rw,noexec,nosuid".to_string(),
        },
        TmpfsMount {
            path: "/var/tmp".to_string(),
            size: "100m".to_string(),
            permissions: "rw,noexec,nosuid".to_string(),
        }
    ]
}
```

### Default Resource Limits
```rust
ResourceLimits {
    memory_limit: "512m".to_string(),
    memory_high: "256m".to_string(),
    memory_swap: "0".to_string(), // No swap
    cpu_limit: 1.0,
    pids_limit: 1000,
    open_files_limit: 65536,
}
```

### Default Health Monitor
```rust
HealthMonitor {
    endpoint: "/health".to_string(),
    interval_seconds: 30,
    timeout_seconds: 10,
    retries: 3,
    start_period_seconds: 40,
}
```

## Configuration Serialization

The data model supports serialization to/from:
- **TOML**: For configuration files
- **Environment Variables**: For runtime injection
- **JSON**: For API responses and programmatic access

## Error Handling

### Validation Errors
- `InvalidConfiguration`: Invalid configuration values
- `SecurityViolation`: Security policy violations
- `ResourceLimitExceeded`: Resource constraints violated
- `ValidationError`: General validation failure

### Runtime Errors
- `ContainerBuildFailed`: Image build failure
- `SecurityScanFailed`: Security scanning failure
- `HealthCheckFailed`: Container health check failure
- `ResourceLimitViolation`: Runtime resource limit exceeded

This data model provides a comprehensive foundation for implementing hardened Docker deployment of Merlin while maintaining security, performance, and operational requirements.