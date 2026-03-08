# Disaster Recovery for Merlin AI Router

## Overview

This directory contains comprehensive disaster recovery procedures and automation scripts for the Merlin AI Router deployment. The system is designed to handle various failure scenarios with automated failover, backup restoration, and recovery procedures.

## Components

### dr-plan.yaml
Comprehensive disaster recovery plan document that includes:
- Recovery objectives (RTO/RPO)
- System classification and dependencies
- Disaster scenarios and response procedures
- Communication plans and stakeholder notifications
- Testing and training requirements
- Continuous improvement processes

### failover.sh
Automated failover script that handles:
- Health monitoring of primary and secondary regions
- Automated failover decision making
- Manual failover capabilities
- DNS updates and service discovery
- Graceful degradation procedures
- Failover verification and testing

## Disaster Recovery Architecture

### Multi-Region Setup
- **Primary Region**: Active production environment
- **Secondary Region**: Standby disaster recovery site
- **Health Monitoring**: Continuous health checks between regions
- **Automated Failover**: Automatic detection and response to failures

### Recovery Objectives

| System Component | RTO | RPO | Criticality |
|-----------------|-----|-----|------------|
| Merlin API | 15 minutes | 5 minutes | Critical |
| Redis Cache | 15 minutes | 5 minutes | Critical |
| Kubernetes Cluster | 1 hour | 15 minutes | Critical |
| Monitoring | 1 hour | 15 minutes | High |

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PRIMARY_REGION` | us-east-1 | Primary AWS/GCP region |
| `SECONDARY_REGION` | us-west-2 | Secondary/failover region |
| `NAMESPACE` | merlin-system | Kubernetes namespace |
| `HEALTH_CHECK_INTERVAL` | 30 | Health check interval in seconds |
| `MAX_RETRIES` | 3 | Maximum retry attempts for failover |
| `LOG_FILE` | /var/log/merlin/failover.log | Failover log file location |

### Usage

#### Health Monitoring
```bash
# Start continuous health monitoring
./scripts/disaster-recovery/failover.sh monitor

# Check primary region health
./scripts/disaster-recovery/failover.sh health-primary

# Check secondary region health
./scripts/disaster-recovery/failover.sh health-secondary

# Get current status
./scripts/disaster-recovery/failover.sh status
```

#### Automated Failover
```bash
# Run automated failover decision process
./scripts/disaster-recovery/failover.sh automated
```

#### Manual Failover
```bash
# Manual failover to secondary region
./scripts/disaster-recovery/failover.sh manual us-west-2
```

## Disaster Scenarios

### Pod Failure
- **Impact**: Medium
- **Response**: Automatic pod restart via Kubernetes
- **Recovery Time**: < 5 minutes
- **Automated Response**: Yes

### Node Failure
- **Impact**: High
- **Response**: Node drain and pod rescheduling
- **Recovery Time**: < 15 minutes
- **Automated Response**: Yes

### Data Corruption
- **Impact**: Critical
- **Response**: Backup restoration from clean backup
- **Recovery Time**: < 30 minutes
- **Automated Response**: Partial (requires validation)

### Security Incident
- **Impact**: Critical
- **Response**: Isolation, forensics, and clean restore
- **Recovery Time**: < 2 hours
- **Automated Response**: No (manual intervention required)

### Region Failure
- **Impact**: Critical
- **Response**: Cross-region failover
- **Recovery Time**: < 1 hour
- **Automated Response**: Yes

## Testing Procedures

### Automated Testing
```bash
# Test backup and restore procedures
./scripts/backup/backup.sh
./scripts/backup/restore.sh /path/to/backup.tar.gz

# Test failover procedures (staging environment only)
./scripts/disaster-recovery/failover.sh manual staging-region

# Test health monitoring
./scripts/disaster-recovery/failover.sh health-primary
```

### Testing Schedule
- **Weekly**: Backup and restore tests
- **Monthly**: Simulation tests
- **Quarterly**: Failover tests
- **Annually**: Full disaster recovery tests

## Monitoring and Alerting

### Health Checks
- **API Health**: HTTP health endpoint checks
- **Kubernetes Health**: Cluster and pod status monitoring
- **Resource Health**: CPU, memory, and storage monitoring
- **Network Health**: Connectivity and latency monitoring

### Alerting
- **Critical Alerts**: Immediate notification for critical failures
- **Warning Alerts**: Notification for degraded performance
- **Information Alerts**: Notification for routine operations
- **Escalation**: Automatic escalation for unresolved issues

### Alert Destinations
- **PagerDuty**: Critical operational alerts
- **Slack**: Team communication and coordination
- **Email**: Management and stakeholder notifications
- **Status Page**: Customer-facing status updates

## Communication Plan

### Stakeholder Communication

| Stakeholder | Notification Method | Escalation Time |
|-------------|-------------------|------------------|
| Operations Team | PagerDuty | 30 minutes |
| Security Team | Slack + Email | 15 minutes |
| Management | Email | 1 hour |
| Customers | Status Page | N/A |

### Communication Templates

#### Initial Incident Notification
```
INCIDENT DECLARED: [Incident Type]
Impact: [Impact Level]
Start Time: [Timestamp]
Estimated Duration: [Duration]
Next Update: [Update Time]
```

#### Progress Update
```
INCIDENT UPDATE: [Incident ID]
Status: [Current Status]
Actions Taken: [Completed Actions]
Next Steps: [Planned Actions]
Next Update: [Update Time]
```

#### Resolution Notification
```
INCIDENT RESOLVED: [Incident ID]
Resolution Time: [Timestamp]
Root Cause: [Root Cause Analysis]
Preventive Measures: [Action Items]
Service Status: Normal
```

## Backup Integration

### Backup Integration Points
- **Automated Backups**: Integration with backup scripts
- **Backup Validation**: Verification of backup integrity
- **Restore Procedures**: Automated restore from backup
- **Cross-Region Replication**: Backup replication between regions

### Backup Testing
- **Weekly**: Backup creation and validation
- **Monthly**: Backup restoration testing
- **Quarterly**: Cross-region backup replication testing

## Security Considerations

### Security Measures
- **Access Control**: Role-based access to failover controls
- **Audit Logging**: Comprehensive logging of all failover activities
- **Encryption**: Encryption of sensitive data in transit and at rest
- **Authentication**: Multi-factor authentication for critical operations

### Security Incident Response
- **Isolation**: Immediate isolation of affected systems
- **Forensics**: Collection and analysis of security data
- **Restoration**: Restoration from clean, verified backups
- **Prevention**: Implementation of security patches and controls

## Performance Optimization

### Performance Monitoring
- **Resource Usage**: CPU, memory, and storage monitoring
- **Response Times**: API response time monitoring
- **Throughput**: Request rate and capacity monitoring
- **Error Rates**: Error rate monitoring and alerting

### Optimization Strategies
- **Resource Scaling**: Automatic scaling based on demand
- **Load Balancing**: Distribution of traffic across instances
- **Caching**: Implementation of caching strategies
- **Connection Pooling**: Optimization of database connections

## Documentation and Training

### Documentation
- **Runbooks**: Step-by-step procedures for common scenarios
- **Architecture Diagrams**: Visual representation of system architecture
- **Configuration Documentation**: Detailed configuration information
- **Training Materials**: Training documentation for team members

### Training Requirements
- **Operations Team**: Quarterly training on DR procedures
- **Management Team**: Annual training on DR overview
- **New Team Members**: Onboarding training on DR processes

## Continuous Improvement

### Post-Incident Reviews
- **Timeline**: Within 1 week of incident resolution
- **Participants**: All relevant stakeholders
- **Focus Areas**: Root cause analysis and prevention
- **Action Items**: Specific improvement tasks

### Metrics and KPIs
- **Response Time**: Time to initial response
- **Resolution Time**: Time to full resolution
- **RTO/RPO Compliance**: Percentage of incidents meeting objectives
- **Customer Impact**: Customer-reported issues and satisfaction

## Troubleshooting

### Common Issues

#### Failover Script Issues
```bash
# Check script permissions
chmod +x scripts/disaster-recovery/failover.sh

# Check environment variables
env | grep -E "(PRIMARY_REGION|SECONDARY_REGION|NAMESPACE)"

# Check log files
tail -f /var/log/merlin/failover.log
```

#### Health Check Failures
```bash
# Test API health manually
curl http://merlin-api-primary.region/health

# Check Kubernetes connectivity
kubectl cluster-info

# Check pod status
kubectl get pods -n merlin-system
```

#### DNS Update Issues
```bash
# Check DNS configuration
kubectl get ingress -n merlin-system

# Test DNS resolution
nslookup merlin-api.example.com

# Check DNS provider status
# (Provider-specific commands)
```

### Debug Commands

```bash
# Validate YAML configuration
yamllint scripts/disaster-recovery/dr-plan.yaml

# Test script syntax
bash -n scripts/disaster-recovery/failover.sh

# Check Kubernetes contexts
kubectl config get-contexts

# Test backup script
./scripts/backup/backup.sh --dry-run
```

## Best Practices

### Prevention
- **Regular Testing**: Regular testing of all DR procedures
- **Monitoring**: Comprehensive monitoring of all system components
- **Documentation**: Up-to-date documentation of all procedures
- **Training**: Regular training for all team members

### Response
- **Quick Response**: Rapid response to all incidents
- **Communication**: Clear and timely communication
- **Documentation**: Detailed documentation of all incidents
- **Follow-up**: Post-incident reviews and improvement

### Recovery
- **Validation**: Thorough validation of all recovery procedures
- **Testing**: Testing of all recovered systems
- **Communication**: Communication of recovery status
- **Improvement**: Continuous improvement of procedures

## Support

For issues or questions regarding disaster recovery:
1. Check the troubleshooting section above
2. Review the disaster recovery plan documentation
3. Contact the operations team for operational issues
4. Contact the security team for security incidents
5. Escalate to management for major incidents