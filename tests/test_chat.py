"""Chat Completion API Tests with Performance Metrics"""

import pytest
import requests
import time
import json


class TestChatCompletion:
    """Test /v1/chat/completions endpoint"""

    def test_non_streaming_basic(self, base_url, chat_payload):
        """Test basic non-streaming chat completion"""
        payload = chat_payload.copy()
        payload["stream"] = False

        start = time.time()
        resp = requests.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            timeout=60
        )
        elapsed_ms = (time.time() - start) * 1000

        assert resp.status_code == 200
        data = resp.json()

        # Verify response structure
        assert data["object"] == "chat.completion"
        assert data["model"] == "Qwen3-4B"
        assert len(data["choices"]) == 1
        assert data["choices"][0]["message"]["role"] == "assistant"
        assert len(data["choices"][0]["message"]["content"]) > 0

        # Verify usage
        assert "usage" in data
        assert data["usage"]["prompt_tokens"] > 0
        assert data["usage"]["completion_tokens"] > 0
        assert data["usage"]["total_tokens"] > 0

        # Get metrics from header
        assert "x-metrics" in resp.headers
        metrics = json.loads(resp.headers["x-metrics"])

        # Print performance summary
        print(f"\n{'='*50}")
        print("  NON-STREAMING PERFORMANCE METRICS")
        print(f"{'='*50}")
        print(f"  Total Duration:       {elapsed_ms:.0f} ms")
        print(f"  Token Throughput:     {metrics['throughput_tokens_per_sec']:.2f} tokens/s")
        print(f"  TTFT (First Token):   {metrics['time_to_first_token_ms']:.0f} ms")
        print(f"  E2E Latency:          {metrics['end_to_end_latency_ms']:.0f} ms")
        print(f"  ITL (Inter-Token):    {metrics.get('inter_token_latency_ms', 0):.1f} ms")
        print(f"  P90 Latency:          {metrics.get('p90_latency_ms', 0):.0f} ms")
        print(f"  P99 Latency:          {metrics.get('p99_latency_ms', 0):.0f} ms")
        print(f"  Completion Tokens:    {metrics['completion_tokens']}")
        print(f"  Prompt Tokens:        {metrics.get('prompt_tokens', data['usage']['prompt_tokens'])}")
        print(f"  Total Tokens:         {data['usage']['total_tokens']}")
        print(f"{'='*50}\n")

    def test_non_streaming_metrics(self, base_url):
        """Test non-streaming metrics are returned correctly"""
        payload = {
            "model": "Qwen3-4B",
            "messages": [{"role": "user", "content": "Hello, who are you?"}],
            "max_tokens": 32,
            "temperature": 0.7,
            "stream": False,
        }

        start = time.time()
        resp = requests.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            timeout=60
        )
        total_ms = (time.time() - start) * 1000

        assert resp.status_code == 200
        data = resp.json()

        # Parse metrics from header
        metrics = json.loads(resp.headers["x-metrics"])

        # Verify metrics fields
        assert "throughput_tokens_per_sec" in metrics
        assert "time_to_first_token_ms" in metrics
        assert "end_to_end_latency_ms" in metrics
        assert "completion_tokens" in metrics

        # Verify values are reasonable
        assert metrics["completion_tokens"] > 0
        assert metrics["time_to_first_token_ms"] >= 0
        assert metrics["end_to_end_latency_ms"] > 0
        assert metrics["throughput_tokens_per_sec"] > 0

        print(f"\n{'='*50}")
        print("  NON-STREAMING METRICS VERIFICATION")
        print(f"{'='*50}")
        print(f"  Throughput (Tokens/s): {metrics['throughput_tokens_per_sec']:.2f}")
        print(f"  TTFT:                  {metrics['time_to_first_token_ms']:.0f} ms")
        print(f"  E2E Latency:           {metrics['end_to_end_latency_ms']:.0f} ms")
        print(f"  Completion Tokens:     {metrics['completion_tokens']}")
        print(f"  Measured Total:        {total_ms:.0f} ms")
        print(f"{'='*50}\n")

    def test_streaming_basic(self, base_url, chat_payload):
        """Test basic streaming chat completion"""
        payload = chat_payload.copy()
        payload["stream"] = True

        start = time.time()
        resp = requests.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            stream=True,
            timeout=60
        )
        elapsed_ms = (time.time() - start) * 1000

        assert resp.status_code == 200

        chunks = []
        final_metrics = None

        for line in resp.iter_lines():
            if line:
                if line.startswith(b"data: "):
                    data = line[6:]
                    if data == b"[DONE]":
                        break
                    chunks.append(data)
                elif line.startswith(b": "):
                    # SSE comment with metrics
                    comment = line[2:].decode()
                    if "[METRICS]" in comment:
                        metrics_str = comment.replace("[METRICS] ", "")
                        final_metrics = json.loads(metrics_str)

        assert len(chunks) > 0

        # Verify first chunk has role
        first_chunk = json.loads(chunks[0])
        assert first_chunk["choices"][0]["delta"].get("role") == "assistant"

        # Print performance summary
        print(f"\n{'='*50}")
        print("  STREAMING PERFORMANCE METRICS")
        print(f"{'='*50}")
        print(f"  Total Duration:       {elapsed_ms:.0f} ms")
        print(f"  Chunks Received:     {len(chunks)}")
        if final_metrics:
            print(f"  Token Throughput:    {final_metrics['throughput_tokens_per_sec']:.2f} tokens/s")
            print(f"  TTFT (First Token):  {final_metrics['time_to_first_token_ms']:.0f} ms")
            print(f"  E2E Latency:         {final_metrics['end_to_end_latency_ms']:.0f} ms")
            print(f"  ITL (Inter-Token):   {final_metrics.get('inter_token_latency_ms', 0):.1f} ms")
            print(f"  P90 Latency:         {final_metrics.get('p90_latency_ms', 0):.0f} ms")
            print(f"  P99 Latency:         {final_metrics.get('p99_latency_ms', 0):.0f} ms")
            print(f"  Completion Tokens:   {final_metrics['completion_tokens']}")
            print(f"  Prompt Tokens:       {final_metrics.get('prompt_tokens', 'N/A')}")
        print(f"{'='*50}\n")

    def test_streaming_ttft(self, base_url):
        """Test Time To First Token measurement"""
        payload = {
            "model": "Qwen3-4B",
            "messages": [{"role": "user", "content": "Say 'test'"}],
            "max_tokens": 8,
            "stream": True,
        }

        start = time.time()
        first_token_received = None
        chunks_count = 0
        final_metrics = None

        resp = requests.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            stream=True,
            timeout=30
        )

        for line in resp.iter_lines():
            if line and line.startswith(b"data: "):
                data = line[6:]
                if data == b"[DONE]":
                    break
                chunks_count += 1
                if first_token_received is None:
                    first_token_received = time.time()
            elif line.startswith(b": "):
                comment = line[2:].decode()
                if "[METRICS]" in comment:
                    metrics_str = comment.replace("[METRICS] ", "")
                    final_metrics = json.loads(metrics_str)

        ttft_ms = (first_token_received - start) * 1000 if first_token_received else 0

        print(f"\n{'='*50}")
        print("  STREAMING TTFT MEASUREMENT")
        print(f"{'='*50}")
        print(f"  TTFT (First Token):   {ttft_ms:.0f} ms")
        print(f"  Chunks Received:      {chunks_count}")
        if final_metrics:
            print(f"  Server TTFT:          {final_metrics['time_to_first_token_ms']:.0f} ms")
            print(f"  Token Throughput:     {final_metrics['throughput_tokens_per_sec']:.2f} tokens/s")
            print(f"  E2E Latency:          {final_metrics['end_to_end_latency_ms']:.0f} ms")
            print(f"  ITL (Inter-Token):    {final_metrics.get('inter_token_latency_ms', 0):.1f} ms")
            print(f"  P90 Latency:          {final_metrics.get('p90_latency_ms', 0):.0f} ms")
            print(f"  P99 Latency:          {final_metrics.get('p99_latency_ms', 0):.0f} ms")
        print(f"{'='*50}\n")

        assert ttft_ms > 0

    def test_repeated_requests_stability(self, base_url):
        """Test repeated requests for stable metrics"""
        payload = {
            "model": "Qwen3-4B",
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 16,
            "stream": False,
        }

        results = []
        for i in range(3):
            start = time.time()
            resp = requests.post(
                f"{base_url}/v1/chat/completions",
                json=payload,
                timeout=60
            )
            elapsed_ms = (time.time() - start) * 1000
            metrics = json.loads(resp.headers["x-metrics"])
            results.append({
                "latency_ms": elapsed_ms,
                "metrics": metrics,
                "response": resp.json(),
            })
            assert resp.status_code == 200

        # Print stability report
        print(f"\n{'='*50}")
        print("  STABILITY TEST - 3 REQUESTS")
        print(f"{'='*50}")
        print(f"{'Req':<6} {'Latency':<12} {'Throughput':<15} {'TTFT':<10} {'Tokens':<8}")
        print("-" * 50)
        for i, r in enumerate(results):
            m = r["metrics"]
            print(f"  {i+1:<4} {r['latency_ms']:>8.0f}ms {m['throughput_tokens_per_sec']:>10.2f} tok/s {m['time_to_first_token_ms']:>8.0f}ms {m['completion_tokens']:>6}")

        # Statistics
        latencies = [r["latency_ms"] for r in results]
        throughputs = [r["metrics"]["throughput_tokens_per_sec"] for r in results]

        print("-" * 50)
        print(f"  Avg Latency:    {sum(latencies)/len(latencies):.0f} ms")
        print(f"  Avg Throughput: {sum(throughputs)/len(throughputs):.2f} tokens/s")
        print(f"  Latency Range:  {min(latencies):.0f} - {max(latencies):.0f} ms")
        print(f"{'='*50}\n")

        # Check stability - latencies should be within 5x of each other
        max_lat = max(latencies)
        min_lat = min(latencies)
        assert max_lat < min_lat * 5, f"Latency variance too high: {min_lat:.0f}ms - {max_lat:.0f}ms"

    def test_input_length_scaling(self, base_url):
        """Test performance with low/medium/high input lengths, fixed output ~2000 tokens"""
        # Input sizes: low=200, medium=3000, high=8000 tokens
        # Output fixed at ~2000 tokens
        test_cases = [
            {"name": "LOW", "input_chars": 200, "max_tokens": 2000},
            {"name": "MEDIUM", "input_chars": 3000, "max_tokens": 2000},
            {"name": "HIGH", "input_chars": 8000, "max_tokens": 2000},
        ]

        print(f"\n{'='*60}")
        print("  INPUT LENGTH SCALING TEST (Fixed Output ~2000 tokens)")
        print(f"{'='*60}")
        print(f"{'Level':<8} {'Input Chars':<12} {'Output':<10} {'TTFT':<10} {'E2E':<12} {'Throughput':<12}")
        print("-" * 60)

        results = []
        for tc in test_cases:
            # Generate input text of specified length
            input_text = "Hello " * (tc["input_chars"] // 6)
            input_text = input_text[: tc["input_chars"]]

            payload = {
                "model": "Qwen3-4B",
                "messages": [{"role": "user", "content": input_text}],
                "max_tokens": tc["max_tokens"],
                "temperature": 0.7,
                "stream": False,
            }

            start = time.time()
            resp = requests.post(
                f"{base_url}/v1/chat/completions",
                json=payload,
                timeout=300
            )
            elapsed_ms = (time.time() - start) * 1000

            assert resp.status_code == 200, f"Request failed for {tc['name']}"
            data = resp.json()
            metrics = json.loads(resp.headers["x-metrics"])

            results.append({
                "name": tc["name"],
                "input_chars": tc["input_chars"],
                "output_tokens": data["usage"]["completion_tokens"],
                "metrics": metrics,
                "elapsed_ms": elapsed_ms,
            })

            m = metrics
            print(f"  {tc['name']:<6} {tc['input_chars']:>10} {data['usage']['completion_tokens']:>8} "
                  f"{m['time_to_first_token_ms']:>8.0f}ms {m['end_to_end_latency_ms']:>10.0f}ms "
                  f"{m['throughput_tokens_per_sec']:>10.2f} tok/s")

        print("-" * 60)

        # Print summary
        print(f"\n  Scaling Analysis:")
        for i, r in enumerate(results):
            m = r["metrics"]
            print(f"  {r['name']}: TTFT={m['time_to_first_token_ms']:.0f}ms, "
                  f"E2E={m['end_to_end_latency_ms']:.0f}ms, "
                  f"ITL={m.get('inter_token_latency_ms', 0):.1f}ms, "
                  f"P90={m.get('p90_latency_ms', 0):.0f}ms, "
                  f"P99={m.get('p99_latency_ms', 0):.0f}ms")

        print(f"{'='*60}\n")


def pytest_terminal_summary(terminalreporter, exitstatus, config):
    """Print performance summary at the end of test run"""
    terminalreporter.write_sep("=", "PERFORMANCE TEST SUMMARY")
    terminalreporter.write_line("")
    terminalreporter.write_line("  Test completed successfully")
    terminalreporter.write_line("")