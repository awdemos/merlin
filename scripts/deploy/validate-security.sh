#!/bin/bash

# Container security validation script
# Validates hardened container security configurations

set -euo pipefail

CONTAINER_NAME=${1:-"merlin-hardened"}
IMAGE_NAME=${2:-"merlin:hardened"}

echo "🔍 Validating security for container: $CONTAINER_NAME"
echo "📦 Image: $IMAGE_NAME"

# Check if container exists
if ! docker ps -a --format 'table {{.Names}}' | grep -q "^$CONTAINER_NAME$"; then
    echo "❌ Container $CONTAINER_NAME not found"
    echo "   Run: docker run -d --name $CONTAINER_NAME $IMAGE_NAME"
    exit 1
fi

# Function to validate security aspect
validate_aspect() {
    local aspect="$1"
    local check="$2"
    local expected="$3"

    echo "🔍 Checking $aspect..."

    if docker inspect "$CONTAINER_NAME" --format="$check" | grep -q "$expected"; then
        echo "✅ $aspect: OK"
        return 0
    else
        echo "❌ $aspect: FAILED"
        return 1
    fi
}

# Security validations
VALIDATIONS_PASSED=0
VALIDATIONS_FAILED=0

# Validate non-root user
if validate_aspect "Non-root user" "{{.Config.User}}" "1000"; then
    ((VALIDATIONS_PASSED++))
else
    ((VALIDATIONS_FAILED++))
fi

# Validate read-only filesystem
if validate_aspect "Read-only filesystem" "{{.HostConfig.ReadonlyRootfs}}" "true"; then
    ((VALIDATIONS_PASSED++))
else
    ((VALIDATIONS_FAILED++))
fi

# Validate no-new-privileges
if validate_aspect "No new privileges" "{{.HostConfig.SecurityOpt}}" "no-new-privileges"; then
    ((VALIDATIONS_PASSED++))
else
    ((VALIDATIONS_FAILED++))
fi

# Validate capability dropping
if docker inspect "$CONTAINER_NAME" --format='{{.HostConfig.CapDrop}}' | grep -q "ALL"; then
    echo "✅ Capability dropping: OK"
    ((VALIDATIONS_PASSED++))
else
    echo "❌ Capability dropping: FAILED"
    ((VALIDATIONS_FAILED++))
fi

# Validate memory limits
if docker inspect "$CONTAINER_NAME" --format='{{.HostConfig.Memory}}' | grep -q "[1-9]"; then
    echo "✅ Memory limits: OK"
    ((VALIDATIONS_PASSED++))
else
    echo "❌ Memory limits: FAILED"
    ((VALIDATIONS_FAILED++))
fi

# Validate network isolation
if docker inspect "$CONTAINER_NAME" --format='{{.HostConfig.NetworkMode}}' | grep -q "none\|internal"; then
    echo "✅ Network isolation: OK"
    ((VALIDATIONS_PASSED++))
else
    echo "⚠️  Network isolation: Using default network"
fi

# Generate security report
echo "📊 Generating security validation report..."
cat > "/tmp/container-security-report-${CONTAINER_NAME}.txt" << EOF
Container Security Validation Report
====================================
Container: $CONTAINER_NAME
Image: $IMAGE_NAME
Date: $(date)

Validation Results:
- Non-root user: $([ $VALIDATIONS_PASSED -gt 0 ] && echo "✅ PASS" || echo "❌ FAIL")
- Read-only filesystem: $([ $VALIDATIONS_PASSED -gt 0 ] && echo "✅ PASS" || echo "❌ FAIL")
- No new privileges: $([ $VALIDATIONS_PASSED -gt 0 ] && echo "✅ PASS" || echo "❌ FAIL")
- Capability dropping: $([ $VALIDATIONS_PASSED -gt 1 ] && echo "✅ PASS" || echo "❌ FAIL")
- Memory limits: $([ $VALIDATIONS_PASSED -gt 1 ] && echo "✅ PASS" || echo "❌ FAIL")
- Network isolation: $(docker inspect "$CONTAINER_NAME" --format='{{.HostConfig.NetworkMode}}')

Summary:
- Tests passed: $VALIDATIONS_PASSED
- Tests failed: $VALIDATIONS_FAILED
- Overall: $([ $VALIDATIONS_FAILED -eq 0 ] && echo "✅ SECURE" || echo "❌ VULNERABLE")

Recommendations:
$([ $VALIDATIONS_FAILED -gt 0 ] && echo "1. Address failed security validations" || echo "1. Container meets security requirements")
$([ $VALIDATIONS_FAILED -gt 0 ] && echo "2. Review Docker security configurations" || echo "2. Monitor container runtime behavior")
$([ $VALIDATIONS_FAILED -gt 0 ] && echo "3. Implement additional hardening measures" || echo "3. Regular security scanning recommended")

EOF

echo "📋 Security report saved to: /tmp/container-security-report-${CONTAINER_NAME}.txt"

# Summary
echo ""
echo "🎯 Security Validation Summary"
echo "=============================="
echo "✅ Passed: $VALIDATIONS_PASSED"
echo "❌ Failed: $VALIDATIONS_FAILED"
echo "📊 Overall: $([ $VALIDATIONS_FAILED -eq 0 ] && echo "SECURE" || echo "VULNERABLE")"

if [ $VALIDATIONS_FAILED -eq 0 ]; then
    echo "🎉 Container meets security requirements!"
    exit 0
else
    echo "⚠️  Container has security vulnerabilities"
    exit 1
fi