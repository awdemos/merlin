# Hardened Docker Deployment for Merlin AI Router

This directory contains Docker-related artifacts for deploying Merlin AI Router in hardened containers with enhanced security isolation.

## Files
- `Dockerfile.hardened` - Multi-stage hardened Dockerfile with non-root user
- `Dockerfile.multi-stage` - Alternative multi-stage build for development
- `security.yml` - Security scanning configuration
- `README.md` - This file

## Security Features
- Non-root user execution (UID 1000)
- Read-only filesystem with tmpfs mounts
- Capability dropping and no-new-privileges
- Resource limits (memory, CPU, PIDs)
- Security scanning integration (Trivy, Hadolint)
- Minimal base images (distroless/alpine)

## Usage
```bash
# Build hardened image
docker build -t merlin:hardened -f Dockerfile.hardened .

# Security scan
trivy image merlin:hardened
hadolint Dockerfile.hardened
```