#!/bin/bash
# Automated test runner for Merlin systemd service manual testing procedure

set -e

echo "Merlin Systemd Service Automated Test Runner"
echo "============================================"

# Test environment setup
export MERLIN_TEST_DIR="/tmp/merlin-test"
export MERLIN_CONFIG_DIR="${MERLIN_TEST_DIR}/etc/merlin"
export MERLIN_DATA_DIR="${MERLIN_TEST_DIR}/var/lib/merlin"
export MERLIN_LOG_DIR="${MERLIN_TEST_DIR}/var/log/merlin"

# Test results tracking
declare -A TEST_RESULTS
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to run a test and track results
run_test() {
    local test_name="$1"
    local test_script="$2"

    echo ""
    echo "Running $test_name..."
    echo "----------------------------------------"

    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    if bash "$test_script"; then
        echo "✓ $test_name PASSED"
        TEST_RESULTS["$test_name"]="PASS"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo "✗ $test_name FAILED"
        TEST_RESULTS["$test_name"]="FAIL"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi

    echo "----------------------------------------"
}

# Function to generate test report
generate_report() {
    echo ""
    echo "Test Report"
    echo "==========="
    echo "Test Date: $(date)"
    echo "Test Environment: $(uname -a)"
    echo ""
    echo "Test Results Summary:"
    echo "- Total Tests: $TOTAL_TESTS"
    echo "- Passed: $PASSED_TESTS"
    echo "- Failed: $FAILED_TESTS"
    echo ""

    if [ $FAILED_TESTS -eq 0 ]; then
        echo "🎉 ALL TESTS PASSED!"
    else
        echo "❌ SOME TESTS FAILED"
        echo ""
        echo "Failed Tests:"
        for test_name in "${!TEST_RESULTS[@]}"; do
            if [ "${TEST_RESULTS[$test_name]}" = "FAIL" ]; then
                echo "- $test_name"
            fi
        done
    fi

    echo ""
    echo "Detailed Results:"
    for test_name in "${!TEST_RESULTS[@]}"; do
        echo "- $test_name: ${TEST_RESULTS[$test_name]}"
    done
}

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "Cleaning up test environment..."

    # Stop and disable service if it's running
    if systemctl is-active merlin >/dev/null 2>&1; then
        sudo systemctl stop merlin
        echo "✓ Service stopped"
    fi

    if systemctl is-enabled merlin >/dev/null 2>&1; then
        sudo systemctl disable merlin
        echo "✓ Service disabled"
    fi

    # Remove service file
    if [ -f /etc/systemd/system/merlin.service ]; then
        sudo rm /etc/systemd/system/merlin.service
        sudo systemctl daemon-reload
        echo "✓ Service file removed"
    fi

    # Remove test environment
    if [ -d "${MERLIN_TEST_DIR}" ]; then
        sudo rm -rf "${MERLIN_TEST_DIR}"
        echo "✓ Test environment cleaned up"
    fi

    generate_report
}

# Set trap for cleanup
trap cleanup EXIT

# Check prerequisites
echo "Checking prerequisites..."

if ! command -v systemctl &> /dev/null; then
    echo "✗ systemctl not found. This test requires systemd."
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "✗ cargo not found. Please install Rust."
    exit 1
fi

if ! command -v curl &> /dev/null; then
    echo "✗ curl not found. Please install curl."
    exit 1
fi

if [ ! -f "scripts/install-systemd.sh" ]; then
    echo "✗ installation script not found. Please run from project root."
    exit 1
fi

echo "✓ All prerequisites found"

# Run tests
run_test "Pre-Installation Tests" "tests/manual/pre-installation.sh"
run_test "Installation Tests" "tests/manual/installation.sh"
run_test "Service Management Tests" "tests/manual/service-management.sh"

# Note: Additional test scripts would be added here as they are created
# run_test "Service Restart Tests" "tests/manual/service-restart.sh"
# run_test "Service Failure Tests" "tests/manual/service-failure.sh"
# run_test "Resource Usage Tests" "tests/manual/resource-usage.sh"
# run_test "Security Tests" "tests/manual/security.sh"
# run_test "Integration Tests" "tests/manual/integration.sh"
# run_test "Performance Tests" "tests/manual/performance.sh"
# run_test "Cleanup Tests" "tests/manual/cleanup.sh"

# Exit with appropriate code
if [ $FAILED_TESTS -eq 0 ]; then
    echo ""
    echo "🎉 All automated tests completed successfully!"
    exit 0
else
    echo ""
    echo "❌ Some tests failed. Please review the output above."
    exit 1
fi