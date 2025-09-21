# Manual Testing Procedure Verification

This document outlines comprehensive manual testing procedures for verifying the systemd service implementation for Merlin AI Router.

## Test Environment Setup

### Prerequisites
- Linux system with systemd (Ubuntu 18.04+, CentOS 7+, or compatible)
- Rust toolchain installed
- Redis server installed and running
- Root/sudo access for service installation

### Test Environment Variables
```bash
export MERLIN_TEST_DIR="/tmp/merlin-test"
export MERLIN_CONFIG_DIR="${MERLIN_TEST_DIR}/etc/merlin"
export MERLIN_DATA_DIR="${MERLIN_TEST_DIR}/var/lib/merlin"
export MERLIN_LOG_DIR="${MERLIN_TEST_DIR}/var/log/merlin"
```

## Pre-Installation Tests

### Test P1.1: System Requirements Verification
```bash
# Verify systemd is available
systemctl --version | head -1

# Verify Rust toolchain
rustc --version
cargo --version

# Verify Redis is available
redis-cli ping

# Verify port availability
ss -tlnp | grep :4242 || echo "Port 4242 is available"
```

**Expected Results:**
- systemd version 230+ available
- Rust 1.75+ installed
- Redis responds with "PONG"
- Port 4242 is available

### Test P1.2: Binary Build Verification
```bash
# Build the release binary
cargo build --release

# Verify binary exists
ls -la target/release/merlin

# Test binary help
./target/release/merlin --help

# Test binary version
./target/release/merlin --version
```

**Expected Results:**
- Binary compiles successfully
- Help command works
- Version information displays correctly

## Installation Tests

### Test I1.1: Installation Script Execution
```bash
# Create test environment
mkdir -p "${MERLIN_TEST_DIR}"

# Run installation script with test prefix
sudo ./scripts/install-systemd.sh --test-prefix "${MERLIN_TEST_DIR}"

# Verify installation completed
echo $?
```

**Expected Results:**
- Installation script completes with exit code 0
- All directories created with correct permissions
- Service files installed in correct locations

### Test I1.2: Directory Structure Verification
```bash
# Verify directory structure
ls -la "${MERLIN_TEST_DIR}/etc/merlin/"
ls -la "${MERLIN_TEST_DIR}/var/lib/merlin/"
ls -la "${MERLIN_TEST_DIR}/var/log/merlin/"

# Verify file permissions
find "${MERLIN_TEST_DIR}" -name "*.service" -exec ls -la {} \;
find "${MERLIN_TEST_DIR}" -name "*.conf" -exec ls -la {} \;
find "${MERLIN_TEST_DIR}" -name "*.env" -exec ls -la {} \;
```

**Expected Results:**
- All directories exist with correct permissions (755 for dirs, 644 for files)
- Configuration files are readable
- Service files are executable

### Test I1.3: Configuration File Validation
```bash
# Test TOML configuration syntax
cargo run -- validate-config "${MERLIN_TEST_DIR}/etc/merlin/merlin.conf"

# Test environment file syntax
source "${MERLIN_TEST_DIR}/etc/merlin/merlin.env"
echo "MERLIN_HTTP_PORT = ${MERLIN_HTTP_PORT}"
echo "REDIS_URL = ${REDIS_URL}"
```

**Expected Results:**
- Configuration validation passes
- Environment variables load correctly
- Port 4242 and Redis URL are properly set

## Service Management Tests

### Test S1.1: Service Installation
```bash
# Install systemd service (copy to systemd directory)
sudo cp "${MERLIN_TEST_DIR}/usr/lib/systemd/system/merlin.service" /etc/systemd/system/

# Reload systemd daemon
sudo systemctl daemon-reload

# Enable service
sudo systemctl enable merlin

# Verify service is enabled
systemctl is-enabled merlin
```

**Expected Results:**
- Service file installs successfully
- Systemd daemon reloads without errors
- Service is enabled to start on boot

### Test S1.2: Service Startup
```bash
# Start service
sudo systemctl start merlin

# Check service status
systemctl status merlin

# Verify service is running
systemctl is-active merlin

# Check service processes
ps aux | grep merlin
```

**Expected Results:**
- Service starts successfully
- Service status shows "active (running)"
- Service process is running under dynamic user
- No error messages in status output

### Test S1.3: Service Functionality
```bash
# Test HTTP endpoint
curl -f http://localhost:4242/health

# Test metrics endpoint
curl -f http://localhost:4242/metrics

# Test API response time
time curl -f http://localhost:4242/health
```

**Expected Results:**
- Health endpoint returns 200 OK
- Metrics endpoint returns 200 OK
- Response time is under 1 second
- JSON response contains proper health status

### Test S1.4: Service Logs
```bash
# View service logs
journalctl -u merlin -n 50

# Check for error messages
journalctl -u merlin -n 50 | grep -i "error\|warn" || echo "No errors found"

# Check for startup messages
journalctl -u merlin -b | grep -i "started\|ready"
```

**Expected Results:**
- Logs show successful startup
- No error or warning messages
- Service indicates readiness in logs

## Service Restart Tests

### Test R1.1: Graceful Restart
```bash
# Note current PID
OLD_PID=$(systemctl show merlin --property MainPID --value)

# Restart service
sudo systemctl restart merlin

# Wait for restart
sleep 5

# Check new PID
NEW_PID=$(systemctl show merlin --property MainPID --value)

# Verify PID changed
echo "Old PID: $OLD_PID, New PID: $NEW_PID"
```

**Expected Results:**
- Service restarts successfully
- PID changes after restart
- Service remains active after restart

### Test R1.2: Restart with Load
```bash
# Generate some load (simulate requests)
for i in {1..10}; do
    curl -f http://localhost:4242/health &
done

# Restart service under load
sudo systemctl restart merlin

# Verify service handles restart gracefully
systemctl status merlin
```

**Expected Results:**
- Service restarts successfully under load
- No crash or failure occurs
- Service returns to active state

## Service Failure Tests

### Test F1.1: Service Crash Recovery
```bash
# Kill the service process
sudo systemctl kill merlin

# Check if service restarts automatically
sleep 10
systemctl status merlin

# Verify restart count
systemctl show merlin --property NRestarts --value
```

**Expected Results:**
- Service automatically restarts after being killed
- Restart count increases
- Service returns to active state

### Test F1.2: Configuration Error Handling
```bash
# Backup original config
sudo cp "${MERLIN_TEST_DIR}/etc/merlin/merlin.conf" "${MERLIN_TEST_DIR}/etc/merlin/merlin.conf.bak"

# Introduce configuration error
sudo bash -c 'echo "invalid_config = true" >> "${MERLIN_TEST_DIR}/etc/merlin/merlin.conf"'

# Try to restart service
sudo systemctl restart merlin

# Check service status
systemctl status merlin

# Check error logs
journalctl -u merlin -n 20

# Restore original config
sudo mv "${MERLIN_TEST_DIR}/etc/merlin/merlin.conf.bak" "${MERLIN_TEST_DIR}/etc/merlin/merlin.conf"
```

**Expected Results:**
- Service fails to start with invalid configuration
- Error logs show configuration parsing error
- Service can be recovered after restoring valid config

## Resource Usage Tests

### Test U1.1: Memory Usage
```bash
# Monitor memory usage
systemctl show merlin --property MemoryCurrent --value

# Check memory limits
systemctl show merlin --property MemoryMax --value

# Monitor over time
for i in {1..5}; do
    echo "Memory usage: $(systemctl show merlin --property MemoryCurrent --value) bytes"
    sleep 2
done
```

**Expected Results:**
- Memory usage stays within limits (512MB max)
- Memory usage is reasonable for service
- No memory leaks detected over time

### Test U1.2: CPU Usage
```bash
# Check CPU scheduling
systemctl show merlin --property CPUSchedulingPolicy --value

# Monitor CPU usage
top -b -n 1 -p $(systemctl show merlin --property MainPID --value)

# Check process priority
ps -p $(systemctl show merlin --property MainPID --value) -o pid,ppid,ni,cmd
```

**Expected Results:**
- CPU scheduling policy is "idle"
- Process priority is lowered (nice value 10)
- CPU usage is minimal during idle periods

## Security Tests

### Test S2.1: User Permissions
```bash
# Check service user
systemctl show merlin --property User --value

# Check process user
ps -p $(systemctl show merlin --property MainPID --value) -o pid,user,cmd

# Verify file permissions
ls -la "${MERLIN_TEST_DIR}/var/lib/merlin/"
```

**Expected Results:**
- Service runs as dynamic user
- Process is not root
- Files are owned by service user

### Test S2.2: Network Restrictions
```bash
# Check listening ports
ss -tlnp | grep merlin

# Verify only port 4242 is listening
netstat -tlnp | grep :4242

# Check network restrictions
systemctl show merlin --property RestrictAddressFamilies --value
```

**Expected Results:**
- Only port 4242 is listening
- No other ports are exposed
- Address family restrictions are in place

## Integration Tests

### Test I2.1: Redis Integration
```bash
# Check Redis connection in logs
journalctl -u merlin -f | grep -i "redis" &
REDIS_LOG_PID=$!

# Test Redis connectivity
redis-cli ping

# Stop Redis
sudo systemctl stop redis

# Check service behavior
sleep 5
systemctl status merlin

# Restart Redis
sudo systemctl start redis

# Check service recovery
sleep 10
systemctl status merlin

# Clean up
kill $REDIS_LOG_PID 2>/dev/null
```

**Expected Results:**
- Service connects to Redis successfully
- Service handles Redis disconnection gracefully
- Service recovers when Redis is restored

### Test I2.2: CLI Integration
```bash
# Test CLI commands with service running
./target/release/merlin --help

# Test feature commands
./target/release/merlin feature list

# Test with custom config
./target/release/merlin --config "${MERLIN_TEST_DIR}/etc/merlin/merlin.conf" feature list
```

**Expected Results:**
- CLI commands work with service running
- Feature management works correctly
- Custom configuration loading works

## Performance Tests

### Test P1.1: Concurrent Connections
```bash
# Generate concurrent connections
for i in {1..50}; do
    curl -f http://localhost:4242/health &
done

# Wait for all requests to complete
wait

# Check service status
systemctl status merlin

# Check logs for errors
journalctl -u merlin -n 20 | grep -i "error" || echo "No errors found"
```

**Expected Results:**
- Service handles 50 concurrent connections
- No connection failures
- Service remains stable

### Test P1.2: Response Time
```bash
# Measure response times
for i in {1..20}; do
    time curl -f http://localhost:4242/health >/dev/null
done 2>&1 | grep real | awk '{print $2}' | sed 's/s//'
```

**Expected Results:**
- Response times are consistent
- Average response time is under 500ms
- No response time outliers

## Cleanup Tests

### Test C1.1: Service Uninstallation
```bash
# Stop service
sudo systemctl stop merlin

# Disable service
sudo systemctl disable merlin

# Remove service file
sudo rm /etc/systemd/system/merlin.service

# Reload systemd
sudo systemctl daemon-reload

# Verify service is removed
systemctl status merlin || echo "Service removed successfully"
```

**Expected Results:**
- Service stops and disables successfully
- Service file is removed
- Systemd daemon reloads without errors

### Test C1.2: Test Environment Cleanup
```bash
# Remove test environment
sudo rm -rf "${MERLIN_TEST_DIR}"

# Verify cleanup
ls -la "${MERLIN_TEST_DIR}" 2>/dev/null || echo "Test environment cleaned up"
```

**Expected Results:**
- Test environment is completely removed
- No leftover files or directories

## Test Reporting

### Test Results Summary
Create a test results summary with the following information:

```bash
# Generate test report
cat << EOF > /tmp/merlin-test-report.txt
Merlin Systemd Service Test Report
===================================

Test Date: $(date)
Test Environment: $(uname -a)

Test Results:
- Pre-Installation Tests: [PASS/FAIL]
- Installation Tests: [PASS/FAIL]
- Service Management Tests: [PASS/FAIL]
- Service Restart Tests: [PASS/FAIL]
- Service Failure Tests: [PASS/FAIL]
- Resource Usage Tests: [PASS/FAIL]
- Security Tests: [PASS/FAIL]
- Integration Tests: [PASS/FAIL]
- Performance Tests: [PASS/FAIL]
- Cleanup Tests: [PASS/FAIL]

Overall Result: [PASS/FAIL]

Notes:
[Additional observations or issues found]
EOF

cat /tmp/merlin-test-report.txt
```

## Known Issues and Limitations

### Known Issues
1. **Redis Dependencies**: Service may fail to start if Redis is not available
2. **Port Conflicts**: Port 4242 must be available on the system
3. **File Permissions**: Some directory permissions may need manual adjustment
4. **System Version Compatibility**: Some systemd features may not be available on older systems

### Limitations
1. **Testing Environment**: Tests assume a clean test environment
2. **Resource Constraints**: Actual resource usage may vary based on load
3. **Network Configuration**: Network restrictions may affect external connectivity
4. **System Configuration**: System-specific configurations may affect behavior

## Troubleshooting

### Common Issues

#### Service Fails to Start
```bash
# Check service status
systemctl status merlin

# View error logs
journalctl -u merlin -n 50

# Check configuration
sudo ./scripts/install-systemd.sh --validate
```

#### Port Already in Use
```bash
# Check port usage
ss -tlnp | grep :4242

# Kill process using port
sudo kill -9 <PID>
```

#### Permission Denied
```bash
# Check file permissions
ls -la /etc/merlin/
ls -la /var/lib/merlin/

# Fix permissions
sudo chown -R merlin:merlin /var/lib/merlin
sudo chmod 755 /var/lib/merlin
```

#### Redis Connection Issues
```bash
# Check Redis status
systemctl status redis

# Test Redis connection
redis-cli ping

# Check Redis logs
journalctl -u redis -n 20
```

## Continuous Testing

For continuous testing, consider creating a test script that automates these procedures:

```bash
#!/bin/bash
# merlin-test.sh - Automated testing script

set -e

echo "Starting Merlin Systemd Service Tests..."

# Run all test sections
./tests/manual/pre-installation.sh
./tests/manual/installation.sh
./tests/manual/service-management.sh
./tests/manual/service-restart.sh
./tests/manual/service-failure.sh
./tests/manual/resource-usage.sh
./tests/manual/security.sh
./tests/manual/integration.sh
./tests/manual/performance.sh
./tests/manual/cleanup.sh

echo "All tests completed successfully!"
```

This comprehensive manual testing procedure ensures that the systemd service implementation is robust, secure, and ready for production deployment.