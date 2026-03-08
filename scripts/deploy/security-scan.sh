#!/bin/bash

# Security scanning script for hardened Docker containers
# Part of Merlin AI Router hardened deployment

set -euo pipefail

IMAGE_NAME=${1:-"merlin:hardened"}
SCAN_DIR=${2:-"$(pwd)/security-scans"}

echo "🔒 Security scanning for image: $IMAGE_NAME"
echo "📁 Scan results directory: $SCAN_DIR"

# Create scan directory
mkdir -p "$SCAN_DIR"

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Trivy vulnerability scan
if command_exists trivy; then
    echo "🔍 Running Trivy vulnerability scan..."
    trivy image --format json --output "$SCAN_DIR/trivy-scan.json" "$IMAGE_NAME"
    trivy image --exit-code 1 --severity CRITICAL "$IMAGE_NAME" || {
        echo "❌ CRITICAL vulnerabilities found!"
        exit 1
    }
    echo "✅ Trivy scan completed"
else
    echo "⚠️  Trivy not found. Install with: brew install trivy"
fi

# Hadolint Dockerfile validation
if command_exists hadolint; then
    echo "🔍 Running Hadolint validation..."
    if [ -f "docker/Dockerfile.hardened" ]; then
        hadolint docker/Dockerfile.hardened > "$SCAN_DIR/hadolint-report.txt" 2>&1 || {
            echo "❌ Dockerfile validation failed!"
            cat "$SCAN_DIR/hadolint-report.txt"
            exit 1
        }
        echo "✅ Hadolint validation passed"
    fi
else
    echo "⚠️  Hadolint not found. Install with: brew install hadolint"
fi

# Docker Bench Security
if command_exists docker; then
    echo "🔍 Running Docker Bench Security..."
    docker run --rm -it \
        -v /var/run/docker.sock:/var/run/docker.sock \
        -v "$SCAN_DIR:/output" \
        docker/docker-bench-security \
        -l /output/bench-security.log 2>/dev/null || true
    echo "✅ Docker Bench Security completed"
fi

# Generate summary report
echo "📊 Generating security summary..."
cat > "$SCAN_DIR/security-summary.txt" << EOF
Security Scan Summary
====================
Image: $IMAGE_NAME
Date: $(date)

Scan Results:
- Trivy vulnerability scan: $(if [ -f "$SCAN_DIR/trivy-scan.json" ]; then echo "✅ Complete"; else echo "❌ Missing"; fi)
- Hadolint validation: $(if [ -f "$SCAN_DIR/hadolint-report.txt" ]; then echo "✅ Complete"; else echo "❌ Missing"; fi)
- Docker Bench Security: $(if [ -f "$SCAN_DIR/bench-security.log" ]; then echo "✅ Complete"; else echo "❌ Missing"; fi)

Next Steps:
1. Review scan results in $SCAN_DIR/
2. Address any CRITICAL vulnerabilities
3. Validate Dockerfile best practices
4. Check container runtime security

EOF

echo "✅ Security scanning completed. Results in $SCAN_DIR/"
echo "📋 Summary: $SCAN_DIR/security-summary.txt"