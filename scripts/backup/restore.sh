#!/bin/bash

# Merlin AI Router Restore Script
# Provides comprehensive restore capabilities from backup archives

set -euo pipefail

# Configuration
BACKUP_DIR="${BACKUP_DIR:-/var/backups/merlin}"
REDIS_HOST="${REDIS_HOST:-localhost}"
REDIS_PORT="${REDIS_PORT:-6379}"
K8S_NAMESPACE="${K8S_NAMESPACE:-merlin-system}"
RESTORE_LOG="${BACKUP_DIR}/restore_$(date +%Y%m%d_%H%M%S).log"

# Logging function
log() {
    local level="$1"
    shift
    local message="$*"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [$level] $message" | tee -a "$RESTORE_LOG"
}

# Error handling
error_exit() {
    log "ERROR" "$1"
    exit 1
}

# Validate backup file
validate_backup() {
    local backup_file="$1"

    log "INFO" "Validating backup file: $backup_file"

    if [[ ! -f "$backup_file" ]]; then
        error_exit "Backup file not found: $backup_file"
    fi

    # Test archive integrity
    if ! tar -tzf "$backup_file" > /dev/null 2>&1; then
        error_exit "Backup archive is corrupted"
    fi

    # Extract and validate manifest
    local temp_dir=$(mktemp -d)
    tar -xzf "$backup_file" -C "$temp_dir"

    local manifest_file=$(find "$temp_dir" -name "backup_manifest.json" | head -1)
    if [[ -z "$manifest_file" ]]; then
        rm -rf "$temp_dir"
        error_exit "Backup manifest not found"
    fi

    # Validate manifest structure
    if ! jq empty "$manifest_file" 2>/dev/null; then
        rm -rf "$temp_dir"
        error_exit "Backup manifest is invalid JSON"
    fi

    log "INFO" "Backup validation completed successfully"
    echo "$temp_dir"
}

# Extract backup
extract_backup() {
    local backup_file="$1"
    local extract_dir="$2"

    log "INFO" "Extracting backup to: $extract_dir"

    mkdir -p "$extract_dir"
    tar -xzf "$backup_file" -C "$extract_dir" || error_exit "Failed to extract backup"

    log "INFO" "Backup extracted successfully"
}

# Restore Kubernetes resources
restore_k8s_resources() {
    local backup_path="$1"
    local resource_file="${backup_path}/k8s_resources.yaml"

    log "INFO" "Restoring Kubernetes resources"

    if [[ ! -f "$resource_file" ]]; then
        log "WARN" "Kubernetes resources file not found: $resource_file"
        return
    fi

    # Create namespace if it doesn't exist
    kubectl create namespace "$K8S_NAMESPACE" --dry-run=client -o yaml | kubectl apply -f - || log "WARN" "Failed to create namespace"

    # Apply resources with strategic merge patch
    kubectl apply -f "$resource_file" || error_exit "Failed to restore Kubernetes resources"

    # Wait for deployments to be ready
    kubectl wait --for=condition=available --timeout=300s deployment -n "$K8S_NAMESPACE" --all || log "WARN" "Timeout waiting for deployments"

    log "INFO" "Kubernetes resources restored successfully"
}

# Restore Redis data
restore_redis_data() {
    local backup_path="$1"
    local redis_dump="${backup_path}/redis_dump.rdb"
    local redis_config="${backup_path}/redis_config.txt"

    log "INFO" "Restoring Redis data"

    # Get Redis pod name
    local redis_pod=$(kubectl get pods -n "$K8S_NAMESPACE" -l app=redis -o jsonpath='{.items[0].metadata.name}' 2>/dev/null)

    if [[ -z "$redis_pod" ]]; then
        log "WARN" "Redis pod not found, skipping Redis restore"
        return
    fi

    # Restore Redis data if available
    if [[ -f "$redis_dump" ]]; then
        # Copy RDB file to Redis pod
        kubectl cp -n "$K8S_NAMESPACE" "$redis_dump" "${redis_pod}:/data/dump.rdb" || log "WARN" "Failed to copy Redis data"

        # Restart Redis to load new data
        kubectl rollout restart -n "$K8S_NAMESPACE" deployment/redis-deployment || log "WARN" "Failed to restart Redis"

        log "INFO" "Redis data restored successfully"
    else
        log "WARN" "Redis dump file not found, skipping Redis data restore"
    fi

    # Restore Redis configuration if available
    if [[ -f "$redis_config" ]]; then
        log "INFO" "Restoring Redis configuration"
        # Apply configuration settings (this would need custom implementation based on your Redis setup)
        log "WARN" "Redis configuration restore not implemented - manual intervention required"
    fi
}

# Restore application configuration
restore_app_config() {
    local backup_path="$1"
    local config_dir="${backup_path}/merlin"

    log "INFO" "Restoring application configuration"

    if [[ -d "$config_dir" ]]; then
        # Restore configuration files
        sudo cp -r "$config_dir" "/etc/" || log "WARN" "Failed to restore application config"

        # Update permissions
        sudo chown -R root:root "/etc/merlin" || log "WARN" "Failed to update config permissions"
        sudo chmod -R 644 "/etc/merlin" || log "WARN" "Failed to update config permissions"

        log "INFO" "Application configuration restored successfully"
    else
        log "WARN" "Application configuration directory not found"
    fi

    # Restore environment variables
    local env_file="${backup_path}/environment_vars.txt"
    if [[ -f "$env_file" ]]; then
        log "INFO" "Environment variables file found - manual review required"
        log "WARN" "Environment variables restore requires manual intervention"
    fi
}

# Restore security configurations
restore_security_config() {
    local backup_path="$1"

    log "INFO" "Restoring security configurations"

    # Restore security policies
    local policies_file="${backup_path}/security_policies.yaml"
    if [[ -f "$policies_file" ]]; then
        sudo cp "$policies_file" "/etc/merlin/policies/" || log "WARN" "Failed to restore security policies"
        log "INFO" "Security policies restored"
    fi

    # Restore audit logs
    local audit_dir="${backup_path}/audit_logs"
    if [[ -d "$audit_dir" ]]; then
        sudo cp -r "$audit_dir" "/var/log/merlin/" || log "WARN" "Failed to restore audit logs"
        log "INFO" "Audit logs restored"
    fi

    # Restore certificates
    local cert_dir="${backup_path}/certificates"
    if [[ -d "$cert_dir" ]]; then
        sudo cp -r "$cert_dir" "/etc/ssl/certs/" || log "WARN" "Failed to restore certificates"
        sudo chown -R root:root "/etc/ssl/certs/merlin" || log "WARN" "Failed to update certificate permissions"
        sudo chmod -R 600 "/etc/ssl/certs/merlin" || log "WARN" "Failed to update certificate permissions"
        log "INFO" "Certificates restored"
    fi

    log "INFO" "Security configurations restoration completed"
}

# Restore logs
restore_logs() {
    local backup_path="$1"
    local logs_dir="${backup_path}/logs"

    log "INFO" "Restoring application logs"

    if [[ -d "$logs_dir" ]]; then
        sudo cp -r "$logs_dir" "/var/log/" || log "WARN" "Failed to restore application logs"
        sudo chown -R root:root "/var/log/merlin" || log "WARN" "Failed to update log permissions"
        log "INFO" "Application logs restored"
    else
        log "WARN" "Logs directory not found"
    fi

    # Restore API logs
    local api_log="${backup_path}/merlin_api.log"
    if [[ -f "$api_log" ]]; then
        sudo cp "$api_log" "/var/log/merlin/" || log "WARN" "Failed to restore API logs"
        log "INFO" "API logs restored"
    fi

    # Restore Redis logs
    local redis_log="${backup_path}/redis.log"
    if [[ -f "$redis_log" ]]; then
        sudo cp "$redis_log" "/var/log/merlin/" || log "WARN" "Failed to restore Redis logs"
        log "INFO" "Redis logs restored"
    fi
}

# Restart services
restart_services() {
    log "INFO" "Restarting services"

    # Restart Merlin API deployment
    kubectl rollout restart -n "$K8S_NAMESPACE" deployment/merlin-api-deployment || log "WARN" "Failed to restart Merlin API"

    # Restart monitoring services
    kubectl rollout restart -n "$K8S_NAMESPACE" deployment/prometheus-deployment || log "WARN" "Failed to restart Prometheus"
    kubectl rollout restart -n "$K8S_NAMESPACE" deployment/grafana-deployment || log "WARN" "Failed to restart Grafana"

    # Wait for services to be ready
    kubectl wait --for=condition=available --timeout=300s deployment -n "$K8S_NAMESPACE" --all || log "WARN" "Timeout waiting for services"

    log "INFO" "Services restarted successfully"
}

# Verify restore
verify_restore() {
    log "INFO" "Verifying restore"

    # Check Kubernetes resources
    kubectl get all -n "$K8S_NAMESPACE" | head -10 || log "WARN" "Failed to verify Kubernetes resources"

    # Check pod status
    local pod_status=$(kubectl get pods -n "$K8S_NAMESPACE" --no-headers | wc -l)
    log "INFO" "Number of running pods: $pod_status"

    # Check API health
    local api_ready=false
    for i in {1..30}; do
        if kubectl exec -n "$K8S_NAMESPACE" deployment/merlin-api-deployment -- wget -qO- http://localhost:8080/health > /dev/null 2>&1; then
            api_ready=true
            break
        fi
        sleep 2
    done

    if [[ "$api_ready" == true ]]; then
        log "INFO" "API health check passed"
    else
        log "WARN" "API health check failed"
    fi

    # Check Redis connectivity
    local redis_ready=false
    for i in {1..30}; do
        if kubectl exec -n "$K8S_NAMESPACE" deployment/redis-deployment -- redis-cli ping > /dev/null 2>&1; then
            redis_ready=true
            break
        fi
        sleep 2
    done

    if [[ "$redis_ready" == true ]]; then
        log "INFO" "Redis connectivity check passed"
    else
        log "WARN" "Redis connectivity check failed"
    fi

    log "INFO" "Restore verification completed"
}

# Send restore notification
send_notification() {
    local status="$1"
    local message="$2"
    local backup_file="$3"

    log "INFO" "Restore status: $status - $message"

    # Add notification logic here (email, Slack, webhook, etc.)
    # Example:
    # curl -X POST -H "Content-Type: application/json" -d "{\"text\":\"Merlin Restore: $status - $message from $backup_file\"}" "$WEBHOOK_URL"

    # For now, just log to system
    logger -t merlin-restore "Restore $status: $message from $backup_file"
}

# List available backups
list_backups() {
    log "INFO" "Available backups:"

    find "$BACKUP_DIR" -name "merlin_backup_*.tar.gz" -type f | sort | while read backup_file; do
        local backup_size=$(du -h "$backup_file" | cut -f1)
        local backup_date=$(date -r "$backup_file" "+%Y-%m-%d %H:%M:%S")
        echo "  $backup_date - $backup_size - $(basename "$backup_file")"
    done
}

# Main restore function
main() {
    local backup_file="$1"

    if [[ -z "$backup_file" ]]; then
        log "ERROR" "No backup file specified"
        echo "Usage: $0 <backup_file>"
        echo "Available backups:"
        list_backups
        exit 1
    fi

    log "INFO" "Starting Merlin AI Router restore"
    log "INFO" "Backup file: $backup_file"
    log "INFO" "K8s namespace: $K8S_NAMESPACE"

    # Validate backup
    local temp_dir=$(validate_backup "$backup_file")
    local backup_name=$(basename "$backup_file" .tar.gz)
    local backup_path="${temp_dir}/${backup_name}"

    # Extract backup
    extract_backup "$backup_file" "$temp_dir"

    # Perform restore operations
    restore_k8s_resources "$backup_path"
    restore_redis_data "$backup_path"
    restore_app_config "$backup_path"
    restore_security_config "$backup_path"
    restore_logs "$backup_path"

    # Restart services
    restart_services

    # Verify restore
    verify_restore

    # Cleanup
    rm -rf "$temp_dir"

    log "INFO" "Restore completed successfully"
    send_notification "SUCCESS" "Restore completed successfully from $backup_file" "$backup_file"
}

# Execute main function
main "$@"