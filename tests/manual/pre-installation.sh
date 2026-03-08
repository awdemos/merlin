#!/bin/bash
# Pre-installation tests for Merlin systemd service

set -e

echo "=== Running Pre-Installation Tests ==="

# Test P1.1: System Requirements Verification
echo "Test P1.1: System Requirements Verification"

# Verify systemd is available
SYSTEMD_VERSION=$(systemctl --version | head -1)
echo "Systemd version: $SYSTEMD_VERSION"

# Verify Rust toolchain
RUSTC_VERSION=$(rustc --version)
CARGO_VERSION=$(cargo --version)
echo "Rust version: $RUSTC_VERSION"
echo "Cargo version: $CARGO_VERSION"

# Verify Redis is available
REDIS_PING=$(redis-cli ping 2>/dev/null || echo "Redis not available")
echo "Redis status: $REDIS_PING"

# Verify port availability
PORT_STATUS=$(ss -tlnp | grep :4242 || echo "Port 4242 is available")
echo "Port 4242 status: $PORT_STATUS"

# Test P1.2: Binary Build Verification
echo "Test P1.2: Binary Build Verification"

# Build the release binary
cargo build --release

# Verify binary exists
if [ -f "target/release/merlin" ]; then
    echo "✓ Binary exists: target/release/merlin"
    ls -la target/release/merlin
else
    echo "✗ Binary not found"
    exit 1
fi

# Test binary help
HELP_OUTPUT=$(./target/release/merlin --help)
if [ $? -eq 0 ]; then
    echo "✓ Help command works"
else
    echo "✗ Help command failed"
    exit 1
fi

# Test binary version
VERSION_OUTPUT=$(./target/release/merlin --version)
if [ $? -eq 0 ]; then
    echo "✓ Version command works: $VERSION_OUTPUT"
else
    echo "✗ Version command failed"
    exit 1
fi

echo "=== Pre-Installation Tests Completed Successfully ==="