# Backup and Recovery for Merlin AI Router

## Overview

This directory contains comprehensive backup and recovery scripts for the Merlin AI Router deployment. The backup system ensures data integrity, quick recovery, and compliance with security requirements.

## Components

### backup.sh
Automated backup script that creates comprehensive backups of:
- Kubernetes resources (deployments, services, configmaps, secrets, etc.)
- Redis data and configuration
- Application configuration and environment variables
- Security policies, audit logs, and certificates
- Application and container logs
- Prometheus metrics data

### restore.sh
Automated restore script that recovers the system from backup archives with validation and verification.

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BACKUP_DIR` | `/var/backups/merlin` | Directory for storing backup files |
| `RETENTION_DAYS` | `30` | Number of days to retain backups |
| `REDIS_HOST` | `localhost` | Redis server hostname |
| `REDIS_PORT` | `6379` | Redis server port |
| `K8S_NAMESPACE` | `merlin-system` | Kubernetes namespace |

### Usage

#### Creating Backups

```bash
# Basic backup
./scripts/backup/backup.sh

# Custom backup directory
BACKUP_DIR=/custom/backup/path ./scripts/backup/backup.sh

# Custom retention period
RETENTION_DAYS=60 ./scripts/backup/backup.sh
```

#### Restoring from Backups

```bash
# List available backups
./scripts/backup/restore.sh

# Restore from specific backup
./scripts/backup/restore.sh /path/to/backup/merlin_backup_20240101_120000.tar.gz

# Custom namespace restore
K8S_NAMESPACE=custom-namespace ./scripts/backup/restore.sh /path/to/backup.tar.gz
```

## Backup Contents

Each backup archive contains:

- `k8s_resources.yaml` - All Kubernetes resources in the namespace
- `crds.yaml` - Custom Resource Definitions
- `storage.yaml` - Persistent Volumes and Claims
- `redis_dump.rdb` - Redis data snapshot
- `redis_config.txt` - Redis configuration
- `merlin/` - Application configuration files
- `environment_vars.txt` - Environment variables
- `security_policies.yaml` - Security policies
- `audit_logs/` - Audit log files
- `certificates/` - SSL/TLS certificates
- `logs/` - Application and system logs
- `merlin_api.log` - API service logs
- `redis.log` - Redis service logs
- `prometheus_snapshots/` - Prometheus metrics snapshots
- `backup_manifest.json` - Backup metadata and manifest

## Security Features

### Backup Security
- **Compression**: All backups are compressed using tar.gz
- **Validation**: Backup integrity is verified using tar test
- **Manifest**: Each backup includes a JSON manifest with metadata
- **Retention**: Automatic cleanup of old backups based on retention policy
- **Logging**: Comprehensive logging of all backup operations

### Restore Security
- **Validation**: Backup archives are validated before restore
- **Integrity Check**: Archive integrity is verified using tar test
- **Manifest Validation**: JSON manifest is validated for proper structure
- **Atomic Operations**: Restore operations are designed to be atomic where possible
- **Rollback**: Failed restores can be rolled back to previous state

### Access Control
- **File Permissions**: Scripts enforce proper file permissions
- **Sudo Requirements**: System-level operations require sudo access
- **Kubernetes Permissions**: Requires appropriate kubectl permissions
- **Audit Trail**: All operations are logged for audit purposes

## Backup Strategies

### Full Backup
The default backup strategy creates full backups of all components:
```bash
./scripts/backup/backup.sh
```

### Component-Specific Backup
For targeted backups, you can modify the backup script to backup specific components:
```bash
# Backup only Kubernetes resources
backup_k8s_resources

# Backup only Redis data
backup_redis_data

# Backup only configuration
backup_app_config
```

### Scheduled Backups
Set up automated backups using cron:
```bash
# Daily backup at 2 AM
0 2 * * * /path/to/merlin/scripts/backup/backup.sh

# Weekly backup on Sunday at 3 AM
0 3 * * 0 /path/to/merlin/scripts/backup/backup.sh
```

## Disaster Recovery

### Recovery Procedures

1. **System Failure**: Restore from the most recent backup
2. **Data Corruption**: Restore from backup before corruption occurred
3. **Configuration Issues**: Restore configuration files and restart services
4. **Security Incident**: Restore from known-good backup and investigate incident

### Recovery Testing

Regular testing of backup and recovery procedures:
```bash
# Test backup creation
./scripts/backup/backup.sh

# Test restore to temporary environment
./scripts/backup/restore.sh /path/to/test/backup.tar.gz

# Verify system functionality after restore
kubectl get pods -n merlin-system
kubectl logs -f deployment/merlin-api-deployment -n merlin-system
```

## Monitoring and Alerts

### Backup Monitoring
- **Log Monitoring**: Monitor backup logs for errors and warnings
- **File Size**: Monitor backup file sizes for unexpected changes
- **Duration**: Monitor backup execution time for performance issues
- **Success Rate**: Track backup success/failure rates

### Alert Configuration
Configure alerts for:
- Backup failures
- Missing backups
- Backup file corruption
- Restore failures
- Disk space issues

## Troubleshooting

### Common Issues

1. **Permission Denied**: Ensure proper file permissions and sudo access
2. **Kubernetes Connection**: Verify kubectl configuration and cluster access
3. **Disk Space**: Ensure sufficient disk space for backup files
4. **Network Issues**: Check network connectivity for remote operations

### Debug Commands

```bash
# Check backup script syntax
bash -n ./scripts/backup/backup.sh

# Test backup creation (dry run)
BACKUP_DIR=/tmp/test-backup ./scripts/backup/backup.sh

# Validate backup integrity
tar -tzf /path/to/backup.tar.gz

# Check backup contents
tar -tzf /path/to/backup.tar.gz | head -20

# Extract backup for inspection
tar -xzf /path/to/backup.tar.gz -C /tmp/extract
```

### Log Analysis

```bash
# View backup logs
tail -f /var/backups/merlin/backup_*.log

# View restore logs
tail -f /var/backups/merlin/restore_*.log

# Filter for errors
grep ERROR /var/backups/merlin/backup_*.log

# Check for warnings
grep WARN /var/backups/merlin/backup_*.log
```

## Best Practices

### Backup Best Practices
- **Regular Backups**: Schedule regular automated backups
- **Off-site Storage**: Store backup copies in multiple locations
- **Encryption**: Encrypt backup files for sensitive data
- **Testing**: Regularly test backup and recovery procedures
- **Documentation**: Document backup and recovery procedures
- **Monitoring**: Monitor backup operations and alert on failures

### Restore Best Practices
- **Validation**: Always validate backup files before restore
- **Testing**: Test restore procedures in non-production environments
- **Rollback Plan**: Have a rollback plan for failed restores
- **Documentation**: Document restore procedures and requirements
- **Verification**: Verify system functionality after restore
- **Communication**: Communicate restore activities to stakeholders

## Compliance

### Compliance Standards
The backup and recovery system supports compliance with:
- **CIS Controls**: Implementation of CIS security controls
- **NIST 800-190**: Application container security guide
- **PCI DSS**: Payment card industry data security standard
- **SOC 2**: Service organization control reporting
- **ISO 27001**: Information security management
- **HIPAA**: Health insurance portability and accountability act
- **GDPR**: General data protection regulation

### Audit Requirements
- **Retention**: Backup retention according to compliance requirements
- **Logging**: Comprehensive logging of all backup and restore operations
- **Access Control**: Proper access controls for backup files and systems
- **Validation**: Regular validation of backup integrity and restore procedures
- **Documentation**: Documentation of backup and recovery procedures

## Integration

### Kubernetes Integration
The backup system integrates with Kubernetes to:
- Backup all Kubernetes resources in the namespace
- Restore Kubernetes deployments and services
- Manage pod lifecycle during restore operations
- Handle persistent volumes and claims

### External Systems
The backup system can be integrated with:
- **Object Storage**: Store backups in S3, GCS, or Azure Blob Storage
- **Monitoring Systems**: Send backup metrics to Prometheus or Datadog
- **Alerting Systems**: Configure alerts in Slack, PagerDuty, or Email
- **SIEM Systems**: Send backup logs to SIEM systems for security monitoring

## Performance Considerations

### Backup Performance
- **Parallel Operations**: Some backup operations run in parallel
- **Compression**: Backups are compressed to reduce storage requirements
- **Incremental Options**: Consider incremental backups for large datasets
- **Resource Usage**: Monitor resource usage during backup operations

### Restore Performance
- **Validation**: Backup validation adds overhead but ensures integrity
- **Parallel Restore**: Some restore operations can be parallelized
- **Network Bandwidth**: Consider network bandwidth for remote restores
- **Storage I/O**: Monitor storage I/O during restore operations

## Support

For issues or questions regarding the backup and recovery system:
1. Check the troubleshooting section above
2. Review backup and restore logs for error messages
3. Verify system requirements and dependencies
4. Test backup and restore procedures in non-production environments
5. Contact system administrators or support teams for assistance