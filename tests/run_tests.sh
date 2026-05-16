#!/bin/bash
# Run Lingbase integration tests with performance metrics

set -e

cd "$(dirname "$0")/.."

echo "======================================"
echo "Lingbase Integration Tests"
echo "======================================"

# Check if server is running
if ! curl -s http://localhost:11017/health > /dev/null 2>&1; then
    echo "Error: Lingbase server is not running on localhost:11017"
    echo "Please start the server first: ./scripts/run.sh"
    exit 1
fi

# Install dependencies if needed
if ! command -v pytest > /dev/null 2>&1; then
    echo "Installing test dependencies..."
    pip install -q -r requirements-test.txt
fi

# Run tests with JSON report
echo ""
echo "Running tests..."
echo ""

pytest tests/ -v -s --tb=short "$@"

echo ""
echo "======================================"
echo "Test run complete"
echo "======================================"