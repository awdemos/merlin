# Systemd Service Configuration - Data Model

## Entities

### ServiceConfiguration
Represents the configuration for running Merlin as a systemd service.

**Fields**:
- `service_mode`: ServiceMode - Operation mode (HTTP server, CLI daemon, hybrid)
- `http_port`: u16 - HTTP server port (default: 8080)
- `bind_address`: String - Network interface to bind (default: "0.0.0.0")
- `log_level`: LogLevel - Logging verbosity (default: Info)
- `data_directory`: String - Path to feature data storage
- `config_directory`: String - Path to configuration files
- `enable_metrics`: bool - Whether to expose metrics endpoint
- `max_connections`: u32 - Maximum concurrent connections
- `request_timeout_seconds`: u64 - HTTP request timeout

**Validation Rules**:
- Port must be within valid range (1-65535)
- Data and config directories must be absolute paths
- Log level must be one of predefined values
- Memory limits must be reasonable for system resources

### ServiceEnvironment
Environment variable configuration for the systemd service.

**Fields**:
- `rust_log`: String - Rust logging level (RUST_LOG)
- `tokio_runtime_threads`: u32 - Tokio worker threads
- `redis_url`: Option<String> - Redis connection URL if used
- `feature_storage_path`: String - Path to features.json
- `systemd_notify`: bool - Enable systemd notifications

**Validation Rules**:
- Rust log format must be valid
- Thread count must be reasonable (1-16)
- Redis URL must be valid if provided

### SecurityConfiguration
Security settings for the systemd service.

**Fields**:
- `dynamic_user`: bool - Use systemd dynamic user
- `no_new_privileges`: bool - Prevent privilege escalation
- `private_devices`: bool - Isolate device access
- `protect_system`: ProtectionLevel - Filesystem protection level
- `read_write_paths`: Vec<String> - Allowed write paths
- `read_only_paths`: Vec<String> - Allowed read paths
- `restrict_address_families`: Vec<String> - Allowed network families

**Validation Rules**:
- Protection level must be valid systemd value
- Paths must be absolute and exist
- Network families must be valid systemd values

### ResourceLimits
Resource constraints for the service.

**Fields**:
- `memory_max`: String - Maximum memory limit (e.g., "512M")
- `memory_high`: String - Memory pressure threshold
- `limit_nofile`: u64 - Maximum open files
- `limit_nproc`: u64 - Maximum processes
- `cpu_shares`: u64 - CPU weight (default: 1024)
- `timeout_start_sec`: u64 - Startup timeout
- `timeout_stop_sec`: u64 - Shutdown timeout

**Validation Rules**:
- Memory limits must use systemd format
- File and process limits must be reasonable
- Timeouts must be sufficient for service startup

### InstallationParameters
Parameters for service installation and setup.

**Fields**:
- `service_name`: String - Systemd service name (default: "merlin")
- `user`: Option<String> - Static user (if not using dynamic user)
- `group`: Option<String> - Static group
- `description`: String - Service description
- `documentation`: Vec<String> - Documentation URLs
- `wants`: Vec<String> - Services to start with this one
- `after`: Vec<String> - Services to start before this one

**Validation Rules**:
- Service name must be valid systemd unit name
- User/group must exist if specified
- Dependencies must be valid systemd services

## Enums

### ServiceMode
```rust
enum ServiceMode {
    HttpServer,    // Run as HTTP API server only
    CliDaemon,     // Run as CLI daemon for command processing
    Hybrid,         // Support both HTTP and CLI operations
}
```

### LogLevel
```rust
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
```

### ProtectionLevel
```rust
enum ProtectionLevel {
    Strict,     // Full system protection
    Full,       # Most directories read-only
    ReadWrite,  # Most directories read-write
    False,      # No protection
}
```

## Relationships

### ServiceConfiguration → ServiceEnvironment (One-to-One)
- Each ServiceConfiguration has one ServiceEnvironment
- Environment variables override configuration defaults
- Used for runtime behavior tuning

### ServiceConfiguration → SecurityConfiguration (One-to-One)
- Each ServiceConfiguration has one SecurityConfiguration
- Security settings apply to the entire service
- Cannot be changed at runtime

### ServiceConfiguration → ResourceLimits (One-to-One)
- Each ServiceConfiguration has one ResourceLimits
- Resource limits prevent system overload
- Can be adjusted via systemd drop-in files

### ServiceConfiguration → InstallationParameters (One-to-One)
- Each ServiceConfiguration has one InstallationParameters
- Installation parameters are used during setup
- Not used during runtime operation

## Configuration Schema

### Service TOML Structure
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

[environment]
rust_log = "info"
tokio_runtime_threads = 4
redis_url = "redis://localhost:6379"
feature_storage_path = "/var/lib/merlin/features.json"
systemd_notify = true

[security]
dynamic_user = true
no_new_privileges = true
private_devices = true
protect_system = "strict"
read_write_paths = ["/var/lib/merlin"]
read_only_paths = ["/etc/merlin"]
restrict_address_families = ["AF_INET", "AF_INET6", "AF_UNIX"]

[resources]
memory_max = "512M"
memory_high = "256M"
limit_nofile = 65536
limit_nproc = 4096
cpu_shares = 1024
timeout_start_sec = 30
timeout_stop_sec = 10

[installation]
service_name = "merlin"
description = "Merlin AI Router Service"
documentation = ["https://github.com/awdemos/merlin"]
wants = ["network.target"]
after = ["network.target", "redis.service"]
```

## Environment Variable Mappings

### Service Configuration via Environment
```bash
# Service mode
export MERLIN_MODE=Hybrid

# HTTP server settings
export MERLIN_HTTP_PORT=8080
export MERLIN_BIND_ADDRESS=0.0.0.0

# Logging
export MERLIN_LOG_LEVEL=Info
export RUST_LOG=info

# Paths
export MERLIN_DATA_DIR=/var/lib/merlin
export MERLIN_CONFIG_DIR=/etc/merlin

# Features
export MERLIN_ENABLE_METRICS=true
export MERLIN_MAX_CONNECTIONS=1000
```

### Security via Environment
```bash
# Resource limits
export MERLIN_MEMORY_MAX=512M
export MERLIN_MEMORY_HIGH=256M
export MERLIN_NOFILE_LIMIT=65536

# Runtime settings
export MERLIN_TOKIO_THREADS=4
export MERLIN_SYSTEMD_NOTIFY=true
```

## Validation Rules

### Service Startup Validation
- Check required directories exist and are accessible
- Validate port availability for HTTP server mode
- Verify configuration file syntax and permissions
- Test data directory read/write permissions
- Validate network connectivity for dependencies

### Resource Validation
- Memory limits must be positive and reasonable
- File limits must be within system constraints
- Thread counts must match available CPU cores
- Timeouts must allow for service initialization

### Security Validation
- Protection levels must be compatible with service requirements
- Path permissions must match access needs
- Network restrictions must allow required protocols
- User permissions must be appropriate for service needs

## Default Values

### Service Defaults
```rust
impl Default for ServiceConfiguration {
    fn default() -> Self {
        Self {
            service_mode: ServiceMode::Hybrid,
            http_port: 8080,
            bind_address: "0.0.0.0".to_string(),
            log_level: LogLevel::Info,
            data_directory: "/var/lib/merlin".to_string(),
            config_directory: "/etc/merlin".to_string(),
            enable_metrics: true,
            max_connections: 1000,
            request_timeout_seconds: 30,
        }
    }
}
```

### Security Defaults
```rust
impl Default for SecurityConfiguration {
    fn default() -> Self {
        Self {
            dynamic_user: true,
            no_new_privileges: true,
            private_devices: true,
            protect_system: ProtectionLevel::Strict,
            read_write_paths: vec!["/var/lib/merlin".to_string()],
            read_only_paths: vec!["/etc/merlin".to_string()],
            restrict_address_families: vec!["AF_INET".to_string(), "AF_INET6".to_string()],
        }
    }
}
```

## Performance Considerations

### Configuration Loading
- Parse TOML configuration on startup
- Override with environment variables
- Validate all settings before service start
- Cache configuration for runtime access

### Hot Reload Support
- Watch configuration file for changes
- Support SIGHUP for configuration reload
- Validate new configuration before applying
- Graceful handling of invalid configurations

### Fallback Behavior
- Use built-in defaults if configuration unavailable
- Log warnings when using fallback values
- Continue service with reduced functionality if possible
- Provide clear error messages for configuration issues

---
**Data Model Complete**: All systemd service configuration entities and validation rules defined.