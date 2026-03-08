# Kubernetes Deployment Manifests for Merlin AI Router

This directory contains comprehensive Kubernetes manifests for deploying the Merlin AI Router with hardened security configurations.

## Architecture Overview

The deployment includes:
- **Merlin AI Router**: Main application with API, metrics, and health endpoints
- **Redis**: Cache and session storage
- **Prometheus**: Metrics collection and alerting
- **Grafana**: Monitoring dashboards and visualization
- **Network Policies**: Security-oriented network segmentation
- **RBAC**: Role-based access control
- **HPA/VPA**: Automatic scaling based on resource usage

## Prerequisites

1. Kubernetes cluster (v1.20+)
2. kubectl configured to access the cluster
3. Docker registry access for merlin-ai-router image
4. (Optional) MetalLB or cloud provider LoadBalancer support

## Quick Start

### 1. Apply Namespaces
```bash
kubectl apply -f namespace.yaml
```

### 2. Apply Configuration
```bash
kubectl apply -f configmap.yaml
kubectl apply -f secret.yaml  # Update secrets first
kubectl apply -f prometheus-config.yaml
```

### 3. Apply RBAC
```bash
kubectl apply -f serviceaccount.yaml
```

### 4. Apply Network Policies
```bash
kubectl apply -f network-policy.yaml
```

### 5. Apply Services
```bash
kubectl apply -f service.yaml
```

### 6. Apply Deployments
```bash
kubectl apply -f deployment.yaml
```

### 7. Apply Autoscaling
```bash
kubectl apply -f hpa.yaml
```

## Configuration

### Secrets Management

Update the `secret.yaml` file with your actual values:
- Redis password
- Docker registry credentials
- API keys
- TLS certificates

### Environment Variables

Key configuration options in `configmap.yaml`:
- `RUST_LOG`: Logging level (debug, info, warn, error)
- `REDIS_HOST`: Redis server address
- `DOCKER_HOST`: Docker daemon socket
- `SECURITY_SCAN_INTERVAL`: Security scanning frequency
- `PROMETHEUS_PORT`: Metrics暴露端口

### Resource Limits

The deployment includes resource limits:
- Memory: 256Mi request, 512Mi limit
- CPU: 250m request, 500m limit

### Scaling Configuration

- **Min replicas**: 3
- **Max replicas**: 10
- **CPU target**: 70% utilization
- **Memory target**: 80% utilization

## Security Features

### Network Security
- Network policies restrict traffic between components
- Default deny-all policy for defense in depth
- Pod anti-affinity for high availability
- Read-only root filesystem
- Non-root user execution

### Access Control
- RBAC with minimal required permissions
- Service account isolation
- Pod security contexts
- Capability dropping

### Monitoring & Logging
- Comprehensive metrics collection
- Audit logging with retention policies
- Health checks with liveness/readiness probes
- Resource usage monitoring

## Monitoring

### Prometheus Metrics

Access Prometheus metrics at:
```
http://merlin-metrics-service:9090/metrics
```

Key metrics:
- Container operations
- Security scan results
- Resource usage
- API response times
- Error rates

### Grafana Dashboards

Access Grafana at:
```
http://grafana-service:3000
```

Default dashboards include:
- System overview
- Resource usage
- Security compliance
- Deployment metrics

### Health Checks

- **Liveness Probe**: `/health` endpoint
- **Readiness Probe**: `/ready` endpoint
- **Metrics**: `/metrics` endpoint

## Backup and Recovery

### Redis Data Backup
```bash
kubectl exec -it redis-deployment-xxx -- redis-cli BGSAVE
kubectl cp redis-deployment-xxx:/data/dump.rdb ./backup/
```

### Configuration Backup
```bash
kubectl get configmaps,secrets -n merlin-system -o yaml > backup.yaml
```

### Restore
```bash
kubectl apply -f backup.yaml
```

## Troubleshooting

### Common Issues

1. **Pods stuck in Pending**: Check resource availability
2. **Image pull errors**: Verify registry access and credentials
3. **Network connectivity**: Check network policies and service discovery
4. **Permission errors**: Verify RBAC configuration

### Debug Commands

```bash
# Check pod status
kubectl get pods -n merlin-system

# View pod logs
kubectl logs -f deployment/merlin-api-deployment -n merlin-system

# Describe pod for details
kubectl describe pod merlin-api-deployment-xxx -n merlin-system

# Check resource usage
kubectl top pods -n merlin-system

# Test network connectivity
kubectl exec -it merlin-api-deployment-xxx -n merlin-system -- wget -qO- http://localhost:8080/health

# Check events
kubectl get events -n merlin-system
```

## Maintenance

### Updates

1. Update the image version in deployment.yaml
2. Apply the changes: `kubectl apply -f deployment.yaml`
3. Monitor rollout status: `kubectl rollout status deployment/merlin-api-deployment`

### Scaling

Manual scaling:
```bash
kubectl scale deployment merlin-api-deployment --replicas=5 -n merlin-system
```

### Cleanup

```bash
kubectl delete -f .
```

## Integration with External Systems

### Docker Registry
Update the image pull secrets in `secret.yaml` for private registries.

### External Redis
Update the Redis service address in `configmap.yaml` if using external Redis.

### External Monitoring
Configure Prometheus to scrape external metrics sources as needed.

## Compliance

This deployment follows:
- Kubernetes security best practices
- CIS Kubernetes Benchmark
- NIST Cybersecurity Framework
- Defense in depth principles
- Least privilege access

## Support

For issues or questions:
1. Check pod logs and events
2. Verify network connectivity
3. Review resource usage
4. Check configuration values
5. Validate secret management