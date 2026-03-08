# Quickstart: Hardened Docker Deployment for Merlin AI Router

This guide provides step-by-step instructions for deploying Merlin AI Router in a hardened Docker container with enhanced security isolation.

## Prerequisites

### System Requirements
- Docker Engine 20.10+ with containerd runtime
- Linux host with kernel 4.0+ for security features
- Minimum 1GB RAM, 2GB recommended
- 100MB disk space for container image

### Security Tools (Recommended)
- Trivy for vulnerability scanning
- Hadolint for Dockerfile validation
- Docker Bench for Security compliance checking

## Quick Deployment

### 1. Build the Hardened Container

```bash
# Clone the repository
git clone https://github.com/your-org/merlin.git
cd merlin

# Build the hardened container image
docker build -t merlin:hardened -f docker/Dockerfile.hardened .

# Verify the image build
docker images | grep merlin
```

### 2. Security Validation

```bash
# Scan for vulnerabilities
trivy image merlin:hardened --exit-code 1 --severity CRITICAL

# Validate Dockerfile best practices
hadolint docker/Dockerfile.hardened

# Run security compliance check
docker run --rm -it docker/docker-bench-security
```

### 3. Deploy the Container

```bash
# Create dedicated network
docker network create merlin-net --internal

# Deploy with security constraints
docker run -d \
  --name merlin-hardened \
  --network merlin-net \
  --pids-limit 100 \
  --memory 512m \
  --memory-swap 512m \
  --cpus 1.0 \
  --read-only \
  --security-opt no-new-privileges \
  --cap-drop ALL \
  --cap-add CHOWN \
  --cap-add SETGID \
  --cap-add SETUID \
  --health-cmd "curl -f http://localhost:4242/health || exit 1" \
  --health-interval 30s \
  --health-timeout 10s \
  --health-retries 3 \
  -v merlin-config:/app/config:ro \
  -v merlin-logs:/app/logs:rw \
  -e MERLIN_ENV=production \
  -e MERLIN_PORT=4242 \
  -e MERLIN_LOG_LEVEL=info \
  merlin:hardened
```

### 4. Verify Deployment

```bash
# Check container status
docker ps | grep merlin-hardened

# Check health status
docker inspect --format='{{.State.Health.Status}}' merlin-hardened

# View logs
docker logs merlin-hardened

# Test API endpoint
curl http://localhost:4242/health
```

## Configuration Management

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `MERLIN_ENV` | Environment (dev/staging/prod) | `development` | Yes |
| `MERLIN_PORT` | Port for API service | `4242` | No |
| `MERLIN_LOG_LEVEL` | Logging level | `info` | No |
| `MERLIN_REDIS_URL` | Redis connection string | - | Yes |
| `MERLIN_MAX_CONCURRENT` | Max concurrent requests | `100` | No |
| `MERLIN_TIMEOUT_SECONDS` | Request timeout | `30` | No |

### Configuration Files

Mount configuration files as read-only volumes:

```bash
# Main configuration
-v $(pwd)/config/merlin.toml:/app/config/merlin.toml:ro

# Security policies
-v $(pwd)/config/security:/app/config/security:ro

# SSL certificates (if needed)
-v $(pwd)/config/ssl:/app/config/ssl:ro
```

### Example TOML Configuration

```toml
[server]
host = "0.0.0.0"
port = 4242
timeout_seconds = 30

[redis]
url = "redis://localhost:6379"
pool_size = 10

[security]
enable_cors = true
allowed_origins = ["https://your-domain.com"]
rate_limit = 100

[logging]
level = "info"
format = "json"
output = "stdout"
```

## Multi-Environment Deployment

### Development

```bash
docker run -d \
  --name merlin-dev \
  -e MERLIN_ENV=development \
  -e MERLIN_LOG_LEVEL=debug \
  -p 4242:4242 \
  -v $(pwd):/app:rw \
  merlin:hardened
```

### Staging

```bash
docker run -d \
  --name merlin-staging \
  --network staging-net \
  -e MERLIN_ENV=staging \
  -e MERLIN_LOG_LEVEL=info \
  --memory 256m \
  --cpus 0.5 \
  merlin:hardened
```

### Production

```bash
docker run -d \
  --name merlin-prod \
  --network prod-net \
  -e MERLIN_ENV=production \
  -e MERLIN_LOG_LEVEL=warn \
  --memory 512m \
  --cpus 1.0 \
  --read-only \
  --security-opt no-new-privileges \
  --cap-drop ALL \
  merlin:hardened
```

## Security Hardening Features

### Container Security

- **Non-root User**: Container runs as dedicated `merlin` user (UID 1000)
- **Read-only Filesystem**: Immutable container filesystem with tmpfs mounts
- **Capability Dropping**: All capabilities dropped except essential ones
- **No New Privileges**: Prevents privilege escalation
- **Resource Limits**: Memory, CPU, and PIDs constraints
- **Network Isolation**: Dedicated internal network

### Security Scanning

- **Vulnerability Scanning**: Automated CVE scanning with Trivy
- **Compliance Checking**: CIS Docker Benchmark compliance
- **Image Hardening**: Minimal base image with signed layers
- **SBOM Generation**: Software Bill of Materials for supply chain security

### Runtime Protection

- **Health Monitoring**: Continuous health checks with auto-restart
- **Resource Monitoring**: Memory and CPU usage tracking
- **Security Logging**: Audit trails for security events
- **Configuration Validation**: Runtime config validation

## Monitoring and Logging

### Health Checks

```bash
# Container health
docker inspect --format='{{json .State.Health}}' merlin-hardened

# API health
curl -f http://localhost:4242/health

# Metrics endpoint
curl -f http://localhost:4242/metrics
```

### Log Management

```bash
# View logs in real-time
docker logs -f merlin-hardened

# Export logs
docker logs merlin-hardened > merlin.log

# Filter by level
docker logs merlin-hardened 2>&1 | grep ERROR
```

### Performance Monitoring

```bash
# Resource usage
docker stats merlin-hardened

# Container inspection
docker inspect merlin-hardened

# Network statistics
docker network inspect merlin-net
```

## Troubleshooting

### Common Issues

#### Container fails to start
```bash
# Check container logs
docker logs merlin-hardened

# Verify configuration
docker run --rm -it \
  -v $(pwd)/config:/app/config:ro \
  merlin:hardened --validate-config
```

#### Security scan failures
```bash
# Scan with detailed output
trivy image --format json --output scan.json merlin:hardened

# Check vulnerabilities
cat scan.json | jq '.Results[].Vulnerabilities'
```

#### Health check failures
```bash
# Test health endpoint manually
docker exec merlin-hardened curl -f http://localhost:4242/health

# Check network connectivity
docker exec merlin-hardened netstat -tlnp
```

### Debug Mode

For troubleshooting, run with reduced security:

```bash
docker run -d \
  --name merlin-debug \
  -e MERLIN_ENV=development \
  -e MERLIN_LOG_LEVEL=debug \
  -p 4242:4242 \
  -v $(pwd):/app:rw \
  --cap-add NET_ADMIN \
  --security-opt apparmor=unconfined \
  merlin:hardened
```

## Maintenance

### Updates and Upgrades

```bash
# Pull latest base image
docker pull gcr.io/distroless/static-debian11:latest

# Rebuild with updated dependencies
docker build --no-cache -t merlin:hardened -f docker/Dockerfile.hardened .

# Rolling update
docker service update --image merlin:hardened merlin-service
```

### Backup and Recovery

```bash
# Backup configuration
docker run --rm \
  -v merlin-config:/source \
  -v $(pwd):/backup \
  alpine tar czf /backup/merlin-config.tar.gz -C /source .

# Restore configuration
docker run --rm \
  -v merlin-config:/target \
  -v $(pwd):/backup \
  alpine tar xzf /backup/merlin-config.tar.gz -C /target
```

## Integration with Orchestration

### Docker Compose

```yaml
version: '3.8'

services:
  merlin:
    image: merlin:hardened
    container_name: merlin
    restart: unless-stopped
    networks:
      - merlin-net
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - SETGID
      - SETUID
    read_only: true
    tmpfs:
      - /tmp:size=100m,exec
      - /var/tmp:size=100m,exec
    environment:
      - MERLIN_ENV=production
      - MERLIN_PORT=4242
    volumes:
      - merlin-config:/app/config:ro
      - merlin-logs:/app/logs:rw
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:4242/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: '1.0'
        reservations:
          memory: 256M
          cpus: '0.5'

  redis:
    image: redis:7-alpine
    container_name: merlin-redis
    networks:
      - merlin-net
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes

networks:
  merlin-net:
    driver: bridge
    internal: true

volumes:
  merlin-config:
  merlin-logs:
  redis-data:
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: merlin
spec:
  replicas: 3
  selector:
    matchLabels:
      app: merlin
  template:
    metadata:
      labels:
        app: merlin
    spec:
      securityContext:
        runAsUser: 1000
        runAsGroup: 1000
        fsGroup: 1000
        readOnlyRootFilesystem: true
      containers:
      - name: merlin
        image: merlin:hardened
        ports:
        - containerPort: 4242
        env:
        - name: MERLIN_ENV
          value: "production"
        - name: MERLIN_PORT
          value: "4242"
        securityContext:
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
            add:
            - CHOWN
            - SETGID
            - SETUID
        resources:
          limits:
            memory: "512Mi"
            cpu: "1"
          requests:
            memory: "256Mi"
            cpu: "0.5"
        livenessProbe:
          httpGet:
            path: /health
            port: 4242
          initialDelaySeconds: 30
          periodSeconds: 30
          timeoutSeconds: 10
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health
            port: 4242
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 1
        volumeMounts:
        - name: config
          mountPath: /app/config
          readOnly: true
        - name: tmp
          mountPath: /tmp
        - name: var-tmp
          mountPath: /var/tmp
      volumes:
      - name: config
        configMap:
          name: merlin-config
      - name: tmp
        emptyDir:
          medium: Memory
      - name: var-tmp
        emptyDir:
          medium: Memory
```

## Next Steps

1. **Customize Configuration**: Modify the TOML configuration files for your specific needs
2. **Set Up Monitoring**: Integrate with your existing monitoring stack (Prometheus, Grafana)
3. **Configure Logging**: Set up log aggregation and alerting
4. **Security Hardening**: Review and enhance security settings based on your environment
5. **Performance Tuning**: Adjust resource limits based on load testing results

For advanced configuration options and integration details, refer to the full documentation in `/specs/004-prefer-hardened-docker/`.