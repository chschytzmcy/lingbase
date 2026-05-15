#!/bin/bash
# Lingbase 测试脚本 - 绕过代理

unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY no_proxy NO_PROXY

HOST="${1:-67.0.0.5}"
PORT="${2:-11017}"
URL="http://${HOST}:${PORT}"

echo "=== Lingbase Health Check ==="
echo "URL: ${URL}"
echo ""

curl -s --noproxy '*' --max-time 10 "${URL}/health" && echo "" && echo ""

echo "=== Backend Info ==="
curl -s --noproxy '*' --max-time 10 "${URL}/v1/models" 2>/dev/null && echo "" && echo ""

echo "=== Chat Completion Test ==="
curl -s --noproxy '*' -X POST "${URL}/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{"model":"Qwen3-4B","stream": true, "messages":[{"role":"user","content":"写一篇800字的文章"}],"max_tokens":1000}' \
  --max-time 30 && echo ""