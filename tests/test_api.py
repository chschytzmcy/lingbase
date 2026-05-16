"""Health Check API Tests"""

import pytest
import requests


def test_health(base_url):
    """Test health endpoint"""
    resp = requests.get(f"{base_url}/health", timeout=10)
    assert resp.status_code == 200
    data = resp.json()
    assert "status" in data or "healthy" in str(data).lower()


def test_models_list(base_url):
    """Test models list endpoint"""
    resp = requests.get(f"{base_url}/v1/models", timeout=10)
    assert resp.status_code == 200
    data = resp.json()
    assert data["object"] == "list"
    assert len(data["data"]) > 0
    assert data["data"][0]["id"] == "Qwen3-4B"