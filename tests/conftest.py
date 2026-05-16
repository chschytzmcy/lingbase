"""Lingbase Integration Tests Configuration"""

import pytest


@pytest.fixture(scope="session")
def base_url():
    """Base URL for Lingbase API"""
    return "http://localhost:11017"


@pytest.fixture(scope="session")
def model_name():
    """Model name for testing"""
    return "Qwen3-4B"


@pytest.fixture
def chat_payload(model_name):
    """Base chat completion payload"""
    return {
        "model": model_name,
        "messages": [{"role": "user", "content": "你好"}],
        "max_tokens": 64,
        "temperature": 0.7,
    }


def pytest_configure(config):
    """Configure pytest"""
    config.addinivalue_line("markers", "metrics: metrics tests")
    config.addinivalue_line("markers", "stream: streaming tests")


@pytest.hookimpl(tryfirst=True)
def pytest_runtest_makereport(item, call):
    """Hook to capture test results for summary"""
    if call.when == "call":
        if not hasattr(item.config, "_results"):
            item.config._results = []
        item.config._results.append({
            "name": item.name,
            "outcome": call.excinfo,
            "duration": call.stop - call.start if hasattr(call, "stop") else 0,
        })


def pytest_terminal_summary(terminalreporter, exitstatus, config):
    """Add custom summary at the end"""
    results = getattr(config, "_results", [])
    if not results:
        return

    passed = sum(1 for r in results if r["outcome"] is None)
    failed = sum(1 for r in results if r["outcome"] is not None)

    terminalreporter.write_sep("=", "Performance Test Summary")

    if passed > 0:
        terminalreporter.write_line(f"  Passed: {passed}", green=True)
    if failed > 0:
        terminalreporter.write_line(f"  Failed: {failed}", red=True)