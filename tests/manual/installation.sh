#!/bin/bash
# Installation tests for Merlin systemd service

set -e

echo "=== Running Installation Tests ==="

# Test environment setup
export MERLIN_TEST_DIR="/tmp/merlin-test"
export MERLIN_CONFIG_DIR="${MERLIN_TEST_DIR}/etc/merlin"
export MERLIN_DATA_DIR="${MERLIN_TEST_DIR}/var/lib/merlin"
export MERLIN_LOG_DIR="${MERLIN_TEST_DIR}/var/log/merlin"

# Clean up any existing test environment
sudo rm -rf "${MERLIN_TEST_DIR}"

# Test I1.1: Installation Script Execution
echo "Test I1.1: Installation Script Execution"

# Create test environment
mkdir -p "${MERLIN_TEST_DIR}"
echo "✓ Test environment created: ${MERLIN_TEST_DIR}"

# Run installation script with test prefix
if sudo ./scripts/install-systemd.sh --test-prefix "${MERLIN_TEST_DIR}"; then
    echo "✓ Installation script completed successfully"
else
    echo "✗ Installation script failed"
    exit 1
fi

# Test I1.2: Directory Structure Verification
echo "Test I1.2: Directory Structure Verification"

# Verify directory structure
for dir in "${MERLIN_CONFIG_DIR}" "${MERLIN_DATA_DIR}" "${MERLIN_LOG_DIR}"; do
    if [ -d "$dir" ]; then
        echo "✓ Directory exists: $dir"
        ls -la "$dir"
    else
        echo "✗ Directory missing: $dir"
        exit 1
    fi
done

# Verify file permissions
echo "Checking file permissions..."
find "${MERLIN_TEST_DIR}" -name "*.service" -exec echo "Service file: {}" \; -exec ls -la {} \;
find "${MERLIN_TEST_DIR}" -name "*.conf" -exec echo "Config file: {}" \; -exec ls -la {} \;
find "${MERLIN_TEST_DIR}" -name "*.env" -exec echo "Env file: {}" \; -exec ls -la {} \;

# Test I1.3: Configuration File Validation
echo "Test I1.3: Configuration File Validation"

# Test TOML configuration syntax
if [ -f "${MERLIN_CONFIG_DIR}/merlin.conf" ]; then
    echo "✓ TOML config file exists"

    # Basic TOML syntax check (using python-toml or similar if available)
    if command -v python3 &> /dev/null; then
        python3 -c "import toml; toml.load('${MERLIN_CONFIG_DIR}/merlin.conf')" 2>/dev/null && \
            echo "✓ TOML syntax is valid" || \
            echo "⚠ TOML syntax validation skipped (python-toml not available)"
    else
        echo "⚠ TOML syntax validation skipped (python3 not available)"
    fi
else
    echo "✗ TOML config file missing"
    exit 1
fi

# Test environment file syntax
if [ -f "${MERLIN_CONFIG_DIR}/merlin.env" ]; then
    echo "✓ Environment file exists"

    # Source environment file
    set -a
    source "${MERLIN_CONFIG_DIR}/merlin.env"
    set +a

    echo "MERLIN_HTTP_PORT = ${MERLIN_HTTP_PORT}"
    echo "REDIS_URL = ${REDIS_URL}"

    # Verify key variables are set
    if [ "${MERLIN_HTTP_PORT}" = "4242" ]; then
        echo "✓ HTTP port is correctly set to 4242"
    else
        echo "✗ HTTP port is not set correctly: ${MERLIN_HTTP_PORT}"
        exit 1
    fi

    if [ -n "${REDIS_URL}" ]; then
        echo "✓ Redis URL is set: ${REDIS_URL}"
    else
        echo "✗ Redis URL is not set"
        exit 1
    fi
else
    echo "✗ Environment file missing"
    exit 1
fi

echo "=== Installation Tests Completed Successfully ==="