# Research: Hardened Docker Deployment for Merlin AI Router

## Base Image Selection

### Decision: Use multi-stage build with distroless base
**Rationale**: Distroless images provide minimal attack surface with only essential dependencies, no package manager, and integrated security scanning support. They're Google-maintained and designed for production workloads.

### Alternatives considered:
- **Alpine**: Small but includes package manager, increasing attack surface
- **Wolfi**: Good security profile but less ecosystem support
- **Ubuntu**: Too large, many unnecessary packages
- **Custom scratch build**: Too complex for maintenance

## Non-Root User Strategy

### Decision: Create dedicated `merlin` user with minimal privileges
**Rationale**: Principle of least privilege - container should run with only necessary permissions. Non-root execution limits damage from container breaches.

**Implementation approach**:
- Create user in Dockerfile with `--system` flag (no login shell)
- Use `USER` directive after binary installation
- Set appropriate file permissions with `COPY --chown`

## Security Scanning Integration

### Decision: Integrate Trivy and Hadolint in CI/CD pipeline
**Rationale**: Automated security scanning ensures compliance with security requirements and catches vulnerabilities early.

**Implementation approach**:
- Trivy for image vulnerability scanning
- Hadolint for Dockerfile best practices
- Fail build on critical vulnerabilities
- Generate SBOM (Software Bill of Materials)

## Resource Limits Strategy

### Decision: Implement comprehensive resource constraints
**Rationale**: Prevents resource exhaustion attacks and ensures predictable performance.

**Implementation approach**:
- Memory limits: 128MB-512MB based on deployment size
- CPU limits: 1.0-2.0 CPU shares
- PIDs limit: 100 processes
- Read-only filesystem with tmpfs for temporary storage

## Configuration Management

### Decision: External configuration via environment variables and mounted volumes
**Rationale**: Supports configuration-driven design principle and enables immutable deployment artifacts.

**Implementation approach**:
- Environment variables for runtime configuration
- Volume mounts for configuration files
- Support for existing TOML configuration patterns
- Configuration validation at container startup

## Health Monitoring Integration

### Decision: Integrate with existing Prometheus monitoring
**Rationale**: Leverages existing observability infrastructure and maintains consistency.

**Implementation approach**:
- Expose container metrics on existing Prometheus endpoint
- Health check endpoint for container orchestration
- Structured logging integration with existing patterns

## Multi-Environment Support

### Decision: Environment-specific build arguments and configuration
**Rationale**: Enables consistent behavior across development, staging, and production environments.

**Implementation approach**:
- Build-time arguments for environment-specific settings
- Configuration profiles for different deployment scenarios
- Environment-specific resource limits and security settings

## Secret Management

### Decision: External secret injection via environment variables
**Rationale**: Aligns with existing constitution requirements and prevents hardcoded secrets.

**Implementation approach**:
- API keys injected via environment variables
- Support for Kubernetes secrets and Docker secrets
- Runtime validation of required secrets

## Compliance & Validation

### Decision: Automated compliance checking with CIS Docker Benchmark
**Rationale**: Ensures container security meets industry standards.

**Implementation approach**:
- Dockerfile follows CIS Docker Benchmark guidelines
- Automated compliance checking in CI/CD
- Security audit logging for compliance reporting

## Integration with Existing Systemd Service

### Decision: Coexistence with systemd deployment option
**Rationale**: Provides deployment flexibility and supports different operational requirements.

**Implementation approach**:
- Clear documentation for both deployment methods
- Shared configuration patterns
- Consistent API endpoints and behavior across deployments

## Performance Considerations

### Decision: Optimize for startup time and runtime efficiency
**Rationale**: Maintains existing performance standards while adding containerization.

**Implementation approach**:
- Multi-stage builds to minimize image size
- Optimized layer ordering for better caching
- Resource limits that don't impact core performance requirements

## Summary

All research areas have been addressed with clear decisions that align with constitutional requirements and best practices for secure container deployment. The approach focuses on security, maintainability, and integration with existing Merlin architecture.