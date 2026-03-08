#!/bin/bash

# Merlin AI Router Backup Script
# Provides comprehensive backup capabilities for configuration, data, and logs

set -euo pipefail

# Configuration
BACKUP_DIR="${BACKUP_DIR:-/var/backups/merlin}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
REDIS_HOST="${REDIS_HOST:-localhost}"
REDIS_PORT="${REDIS_PORT:-6379}"
K8S_NAMESPACE="${K8S_NAMESPACE:-merlin-system}"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_NAME="merlin_backup_${TIMESTAMP}"
BACKUP_PATH="${BACKUP_DIR}/${BACKUP_NAME}"
LOG_FILE="${BACKUP_DIR}/backup_${TIMESTAMP}.log"

# Logging function
log() {
    local level="$1"
    shift
    local message="$*"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [$level] $message" | tee -a "$LOG_FILE"
}

# Error handling
error_exit() {
    log "ERROR" "$1"
    exit 1
}

# Create backup directory
create_backup_dir() {
    log "INFO" "Creating backup directory: $BACKUP_PATH"
    mkdir -p "$BACKUP_PATH" || error_exit "Failed to create backup directory"
}

# Backup Kubernetes resources
backup_k8s_resources() {
    log "INFO" "Backing up Kubernetes resources"

    # Backup all resources in namespace
    kubectl get all,configmap,secret,ingress,networkpolicy,hpa,vpa -n "$K8S_NAMESPACE" -o yaml > "${BACKUP_PATH}/k8s_resources.yaml" 2>/dev/null || error_exit "Failed to backup K8s resources"

    # Backup CRDs
    kubectl get crd -o yaml > "${BACKUP_PATH}/crds.yaml" 2>/dev/null || log "WARN" "Failed to backup CRDs"

    # Backup PV/PVCs
    kubectl get pv,pvc -n "$K8S_NAMESPACE" -o yaml > "${BACKUP_PATH}/storage.yaml" 2>/dev/null || log "WARN" "Failed to backup storage resources"

    log "INFO" "Kubernetes resources backup completed"
}

# Backup Redis data
backup_redis_data() {
    log "INFO" "Backing up Redis data"

    # Get Redis pod name
    REDIS_POD=$(kubectl get pods -n "$K8S_NAMESPACE" -l app=redis -o jsonpath='{.items[0].metadata.name}' 2>/dev/null)

    if [[ -z "$REDIS_POD" ]]; then
        log "WARN" "Redis pod not found, skipping Redis backup"
        return
    fi

    # Create Redis backup
    kubectl exec -n "$K8S_NAMESPACE" "$REDIS_POD" -- redis-cli BGSAVE || log "WARN" "Failed to trigger Redis BGSAVE"

    # Wait for save to complete
    sleep 5

    # Copy Redis data
    kubectl cp -n "$K8S_NAMESPACE" "${REDIS_POD}:/data/dump.rdb" "${BACKUP_PATH}/redis_dump.rdb" 2>/dev/null || log "WARN" "Failed to copy Redis data"

    # Backup Redis configuration
    kubectl exec -n "$K8S_NAMESPACE" "$REDIS_POD" -- redis-cli CONFIG GET "*" > "${BACKUP_PATH}/redis_config.txt" 2>/dev/null || log "WARN" "Failed to backup Redis config"

    log "INFO" "Redis data backup completed"
}

# Backup application configuration
backup_app_config() {
    log "INFO" "Backing up application configuration"

    # Backup configuration files
    if [[ -d "/etc/merlin" ]]; then
        cp -r "/etc/merlin" "${BACKUP_PATH}/" || log "WARN" "Failed to copy application config"
    fi

    # Backup environment variables
    env | grep -E "(MERLIN|REDIS|DOCKER|SECURITY|PROMETHEUS)" > "${BACKUP_PATH}/environment_vars.txt" 2>/dev/null || log "WARN" "Failed to backup environment variables"

    log "INFO" "Application configuration backup completed"
}

# Backup security configurations
backup_security_config() {
    log "INFO" "Backing up security configurations"

    # Backup security policies
    if [[ -f "/etc/merlin/policies/policies.yaml" ]]; then
        cp "/etc/merlin/policies/policies.yaml" "${BACKUP_PATH}/security_policies.yaml" || log "WARN" "Failed to backup security policies"
    fi

    # Backup audit logs
    if [[ -d "/var/log/merlin/audit" ]]; then
        cp -r "/var/log/merlin/audit" "${BACKUP_PATH}/audit_logs/" 2>/dev/null || log "WARN" "Failed to backup audit logs"
    fi

    # Backup certificates
    if [[ -d "/etc/ssl/certs/merlin" ]]; then
        cp -r "/etc/ssl/certs/merlin" "${BACKUP_PATH}/certificates/" 2>/dev/null || log "WARN" "Failed to backup certificates"
    fi

    log "INFO" "Security configurations backup completed"
}

# Backup logs
backup_logs() {
    log "INFO" "Backing up application logs"

    # Backup application logs
    if [[ -d "/var/log/merlin" ]]; then
        # Exclude audit logs (already backed up)
        cp -r "/var/log/merlin" "${BACKUP_PATH}/logs/" 2>/dev/null || log "WARN" "Failed to backup application logs"
        rm -rf "${BACKUP_PATH}/logs/audit" 2>/dev/null || true
    fi

    # Backup container logs
    kubectl logs -n "$K8S_NAMESPACE" deployment/merlin-api-deployment --tail=10000 > "${BACKUP_PATH}/merlin_api.log" 2>/dev/null || log "WARN" "Failed to backup API logs"
    kubectl logs -n "$K8S_NAMESPACE" deployment/redis-deployment --tail=1000 > "${BACKUP_PATH}/redis.log" 2>/dev/null || log "WARN" "Failed to backup Redis logs"

    log "INFO" "Application logs backup completed"
}

# Backup metrics data
backup_metrics() {
    log "INFO" "Backing up metrics data"

    # Backup Prometheus data if available
    PROMETHEUS_POD=$(kubectl get pods -n "$K8S_NAMESPACE" -l app=prometheus -o jsonpath='{.items[0].metadata.name}' 2>/dev/null)

    if [[ -n "$PROMETHEUS_POD" ]]; then
        # Snapshot Prometheus data
        kubectl exec -n "$K8S_NAMESPACE" "$PROMETHEUS_POD" -- curl -XPOST http://localhost:9090/api/v1/admin/tsdb/snapshot 2>/dev/null || log "WARN" "Failed to snapshot Prometheus data"

        # Copy snapshot data
        kubectl cp -n "$K8S_NAMESPACE" "${PROMETHEUS_POD}:/prometheus/snapshots" "${BACKUP_PATH}/prometheus_snapshots/" 2>/dev/null || log "WARN" "Failed to copy Prometheus snapshots"
    fi

    log "INFO" "Metrics data backup completed"
}

# Create backup manifest
create_backup_manifest() {
    log "INFO" "Creating backup manifest"

    cat > "${BACKUP_PATH}/backup_manifest.json" << EOF
{
  "backup_name": "$BACKUP_NAME",
  "timestamp": "$(date -Iseconds)",
  "backup_version": "1.0",
  "components": {
    "k8s_resources": true,
    "redis_data": true,
    "app_config": true,
    "security_config": true,
    "logs": true,
    "metrics": true
  },
  "backup_size_bytes": $(du -sb "$BACKUP_PATH" | cut -f1),
  "compressed": false,
  "encryption": "none"
}
EOF
}

# Compress backup
compress_backup() {
    log "INFO" "Compressing backup"

    cd "$BACKUP_DIR"
    tar -czf "${BACKUP_NAME}.tar.gz" "$BACKUP_NAME" || error_exit "Failed to compress backup"

    # Update manifest with compression info
    if [[ -f "${BACKUP_PATH}/backup_manifest.json" ]]; then
        sed -i 's/"compressed": false/"compressed": true/' "${BACKUP_PATH}/backup_manifest.json"
        sed -i "s/\"backup_size_bytes\": $(du -sb \"$BACKUP_PATH\" | cut -f1)/\"backup_size_bytes\": $(stat -c%s \"${BACKUP_NAME}.tar.gz\")/" "${BACKUP_PATH}/backup_manifest.json"
    fi

    # Remove uncompressed backup
    rm -rf "$BACKUP_PATH"

    log "INFO" "Backup compressed to ${BACKUP_NAME}.tar.gz"
}

# Cleanup old backups
cleanup_old_backups() {
    log "INFO" "Cleaning up backups older than $RETENTION_DAYS days"

    find "$BACKUP_DIR" -name "merlin_backup_*.tar.gz" -mtime +$RETENTION_DAYS -delete || log "WARN" "Failed to cleanup old backups"

    # Clean up old log files
    find "$BACKUP_DIR" -name "backup_*.log" -mtime +$RETENTION_DAYS -delete || log "WARN" "Failed to cleanup old log files"

    log "INFO" "Cleanup completed"
}

# Validate backup
validate_backup() {
    log "INFO" "Validating backup"

    local backup_file="${BACKUP_DIR}/${BACKUP_NAME}.tar.gz"

    if [[ ! -f "$backup_file" ]]; then
        error_exit "Backup file not found: $backup_file"
    fi

    # Test archive integrity
    if ! tar -tzf "$backup_file" > /dev/null 2>&1; then
        error_exit "Backup archive is corrupted"
    fi

    # Check for essential files
    local essential_files=(
        "${BACKUP_NAME}/k8s_resources.yaml"
        "${BACKUP_NAME}/backup_manifest.json"
    )

    for file in "${essential_files[@]}"; do
        if ! tar -tzf "$backup_file" "$file" > /dev/null 2>&1; then
            log "WARN" "Essential file missing from backup: $file"
        fi
    done

    log "INFO" "Backup validation completed"
}

# Send backup notification
send_notification() {
    local status="$1"
    local message="$2"

    log "INFO" "Backup status: $status - $message"

    # Add notification logic here (email, Slack, webhook, etc.)
    # Example:
    # curl -X POST -H "Content-Type: application/json" -d "{\"text\":\"Merlin Backup: $status - $message\"}" "$WEBHOOK_URL"

    # For now, just log to system
    logger -t merlin-backup "Backup $status: $message"
}

# Main backup function
main() {
    log "INFO" "Starting Merlin AI Router backup"
    log "INFO" "Backup directory: $BACKUP_DIR"
    log "INFO" "Retention period: $RETENTION_DAYS days"
    log "INFO" "K8s namespace: $K8S_NAMESPACE"

    # Create backup directory
    create_backup_dir

    # Perform backups
    backup_k8s_resources
    backup_redis_data
    backup_app_config
    backup_security_config
    backup_logs
    backup_metrics

    # Create manifest
    create_backup_manifest

    # Compress backup
    compress_backup

    # Validate backup
    validate_backup

    # Cleanup old backups
    cleanup_old_backups

    local backup_size=$(du -h "${BACKUP_DIR}/${BACKUP_NAME}.tar.gz" | cut -f1)
    log "INFO" "Backup completed successfully. Size: $backup_size"

    send_notification "SUCCESS" "Backup completed successfully. Size: $backup_size"
}

# Execute main function
main "$@"