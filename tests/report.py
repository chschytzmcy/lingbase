"""
Lingbase Performance Test Report Generator

Usage:
    python -m tests.report

Requires pytest-json-report for JSON output:
    pip install pytest-json-report
"""

import json
import sys
from pathlib import Path


def generate_report(json_file: str = "test-results.json"):
    """Generate performance test report from JSON results"""
    if not Path(json_file).exists():
        print(f"Error: {json_file} not found. Run tests with --json-report first.")
        sys.exit(1)

    with open(json_file) as f:
        data = json.load(f)

    print("=" * 60)
    print("  LINGBASE PERFORMANCE TEST REPORT")
    print("=" * 60)

    # Summary
    passed = sum(1 for r in data if r.get("outcome") == "passed")
    failed = sum(1 for r in data if r.get("outcome") == "failed")
    total = len(data)

    print(f"\nTotal Tests: {total}")
    print(f"  Passed: {passed}")
    print(f"  Failed: {failed}")

    # Performance metrics table
    print("\n" + "-" * 60)
    print("  Test Results")
    print("-" * 60)
    print(f"{'Test Name':<45} {'Status':<10} {'Duration':<15}")
    print("-" * 60)

    for r in data:
        status = "PASS" if r.get("outcome") == "passed" else "FAIL"
        duration = r.get("duration", 0)
        name = r.get("nodeid", "").split("::")[-1]
        print(f"{name:<45} {status:<10} {duration*1000:.0f}ms")

    print("-" * 60)

    # Check if all passed
    if failed == 0:
        print("\n✓ All tests passed!")
    else:
        print(f"\n✗ {failed} tests failed")

    return failed == 0


if __name__ == "__main__":
    json_file = sys.argv[1] if len(sys.argv) > 1 else "test-results.json"
    success = generate_report(json_file)
    sys.exit(0 if success else 1)