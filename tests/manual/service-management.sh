#!/bin/bash
# Service management tests for Merlin systemd service

set -e

echo "=== Running Service Management Tests ==="

# Test environment setup
export MERLIN_TEST_DIR="/tmp/merlin-test"
export MERLIN_CONFIG_DIR="${MERLIN_TEST_DIR}/etc/merlin"

# Check if installation exists
if [ ! -f "${MERLIN_CONFIG_DIR}/merlin.conf" ]; then
    echo "✗ Installation not found. Run installation tests first."
    exit 1
fi

# Test S1.1: Service Installation
echo "Test S1.1: Service Installation"

# Check if service file exists
SERVICE_FILE="${MERLIN_TEST_DIR}/usr/lib/systemd/system/merlin.service"
if [ -f "$SERVICE_FILE" ]; then
    echo "✓ Service file exists: $SERVICE_FILE"
else
    echo "✗ Service file not found"
    exit 1
fi

# Install systemd service
sudo cp "$SERVICE_FILE" /etc/systemd/system/
echo "✓ Service file copied to /etc/systemd/system/"

# Reload systemd daemon
sudo systemctl daemon-reload
echo "✓ Systemd daemon reloaded"

# Enable service
sudo systemctl enable merlin
echo "✓ Service enabled"

# Verify service is enabled
if systemctl is-enabled merlin | grep -q "enabled"; then
    echo "✓ Service is enabled to start on boot"
else
    echo "✗ Service is not enabled"
    exit 1
fi

# Test S1.2: Service Startup
echo "Test S1.2: Service Startup"

# Start service
sudo systemctl start merlin
echo "✓ Service start command issued"

# Wait for service to start
sleep 5

# Check service status
SERVICE_STATUS=$(systemctl status merlin | grep -o "Active:.*" | head -1)
echo "Service status: $SERVICE_STATUS"

# Verify service is running
if systemctl is-active merlin | grep -q "active"; then
    echo "✓ Service is active and running"
else
    echo "✗ Service is not active"
    exit 1
fi

# Check service processes
SERVICE_PID=$(systemctl show merlin --property MainPID --value)
if [ -n "$SERVICE_PID" ] && [ "$SERVICE_PID" != "0" ]; then
    echo "✓ Service process found with PID: $SERVICE_PID"
    ps -p "$SERVICE_PID" -o pid,user,cmd
else
    echo "✗ Service process not found"
    exit 1
fi

# Test S1.3: Service Functionality
echo "Test S1.3: Service Functionality"

# Test HTTP endpoint
echo "Testing HTTP health endpoint..."
if curl -f http://localhost:4242/health >/dev/null 2>&1; then
    echo "✓ Health endpoint is accessible"
    curl -s http://localhost:4242/health | head -5
else
    echo "✗ Health endpoint is not accessible"
    exit 1
fi

# Test metrics endpoint
echo "Testing metrics endpoint..."
if curl -f http://localhost:4242/metrics >/dev/null 2>&1; then
    echo "✓ Metrics endpoint is accessible"
else
    echo "⚠ Metrics endpoint is not accessible (may not be enabled)"
fi

# Test API response time
echo "Testing response time..."
RESPONSE_TIME=$(curl -o /dev/null -s -w '%{time_total}' http://localhost:4242/health)
echo "Response time: ${RESPONSE_TIME}s"

if (( $(echo "$RESPONSE_TIME < 1.0" | bc -l) )); then
    echo "✓ Response time is acceptable (< 1s)"
else
    echo "⚠ Response time is slow (> 1s)"
fi

# Test S1.4: Service Logs
echo "Test S1.4: Service Logs"

# View service logs
echo "Recent service logs:"
journalctl -u merlin -n 10

# Check for error messages
ERROR_COUNT=$(journalctl -u merlin -n 50 | grep -i "error\|warn" | wc -l)
echo "Error/Warning messages found: $ERROR_COUNT"

if [ "$ERROR_COUNT" -eq 0 ]; then
    echo "✓ No error or warning messages found"
else
    echo "⚠ Found $ERROR_COUNT error/warning messages"
    journalctl -u merlin -n 50 | grep -i "error\|warn"
fi

# Check for startup messages
if journalctl -u merlin -b | grep -i "started\|ready" | head -1; then
    echo "✓ Service startup messages found"
else
    echo "⚠ No startup messages found"
fi

echo "=== Service Management Tests Completed Successfully ==="