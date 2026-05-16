# Lingbase

边缘 LLM 推理服务 - 基于 llama.cpp 的 OpenAI 兼容 API 服务。

## 特性

- **OpenAI 兼容**: 支持 `/v1/chat/completions` API
- **流式输出**: SSE 实时 token 流式返回
- **跨平台**: 支持 x86/ARM CPU，可扩展 CUDA/RKLLM
- **轻量高效**: 基于 llama.cpp，无重型依赖
- **性能指标**: TTFT、ITL、Throughput、E2E、P90/P99 分位数

## 快速开始

### 1. 安装依赖

```bash
# Rust (需要 1.81+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 构建工具
sudo apt install cmake build-essential
```

### 2. 下载模型

#### 从 ModelScope 下载

```bash
# 安装 modelscope CLI
pip install modelscope

# 下载 Qwen3-4B-GGUF (推荐 Q4_K_M 量化)
modelscope download --model Qwen/Qwen3-4B-GGUF --include "Qwen3-4B-Q4_K_M.gguf"
```

#### 从 HuggingFace 下载

```bash
# 安装 huggingface-cli
pip install huggingface_hub

# 下载模型
huggingface-cli download Qwen/Qwen3-4B-GGUF Qwen3-4B-Q4_K_M.gguf --local-dir ./models
```

### 3. 配置模型路径

修改 `config/environment.toml`：

```toml
[model]
test_model_path = "/home/etsme/.cache/modelscope/hub/models/Qwen/Qwen3-4B-GGUF/Qwen3-4B-Q4_K_M.gguf"
context_size = 16384
max_prompt_tokens = 16384
max_generation_tokens = 2048
```

### 4. 编译运行

```bash
cargo build --release
./scripts/run.sh
```

### 5. 测试 API

```bash
# 非流式请求
curl -X POST http://192.168.0.124:11017/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "Qwen3-4B",
    "messages": [{"role": "user", "content": "你好，介绍一下自己"}],
    "max_tokens": 128
  }'

# 流式请求
curl -X POST http://192.168.0.124:11017/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "Qwen3-4B",
    "messages": [{"role": "user", "content": "你好，介绍一下自己"}],
    "stream": true,
    "max_tokens": 128
  }'

# 健康检查
curl http://localhost:11017/health

# 模型列表
curl http://localhost:11017/v1/models
```

## API 接口

### POST /v1/chat/completions

OpenAI 兼容的聊天补全接口。

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| model | string | 是 | 模型名称 |
| messages | array | 是 | 消息列表 |
| max_tokens | int | 否 | 最大生成 tokens (默认 256) |
| temperature | float | 否 | 温度参数 (默认 0.7) |
| top_p | float | 否 | Top-p sampling (默认 0.9) |
| stream | bool | 否 | 是否流式输出 (默认 false) |

**响应头** (`x-metrics`)：

非流式响应返回性能指标 JSON：
```json
{
  "throughput_tokens_per_sec": 2.06,
  "time_to_first_token_ms": 2991,
  "end_to_end_latency_ms": 9242,
  "completion_tokens": 19,
  "inter_token_latency_ms": 347.3,
  "p90_latency_ms": 407,
  "p99_latency_ms": 411
}
```

**流式响应** (`[METRICS]` comment)：

```
data: {...}
: [METRICS] {"throughput_tokens_per_sec":6.99,"time_to_first_token_ms":514,...}
```

### GET /health

健康检查接口，返回服务状态和模型加载信息。

### GET /v1/models

返回可用模型列表。

## 性能指标

| 指标 | 说明 | 单位 |
|------|------|------|
| TTFT (Time To First Token) | 输入提交到首个输出 token 的时间 | ms |
| Throughput | 每秒生成 token 数，衡量生成流畅度 | tokens/s |
| E2E Latency | 端到端总耗时，从输入发起到完整输出返回 | ms |
| ITL (Inter-Token Latency) | 相邻 token 间平均延迟，衡量生成流畅度 | ms |
| P90/P99 Latency | 90%/99% 请求的时延上限，衡量长尾延迟 | ms |

## 项目结构

```
lingbase/
├── src/
│   ├── api/           # HTTP API 层 (Axum)
│   ├── backend/       # 推理后端抽象 (CPU/CUDA)
│   ├── llama/        # llama.cpp FFI 封装
│   └── infra/        # 基础设施 (配置、日志、健康检查)
├── config/            # 配置文件
├── lib/               # 预编译 llama.cpp 库
├── tests/             # 集成测试 (Python)
└── docs/              # 设计文档
```

## 测试

### 运行测试

```bash
# 安装依赖
pip install requests pytest --break-system-packages

# 运行测试
./tests/run_tests.sh
```

### 测试用例

| 测试 | 说明 |
|------|------|
| test_health | 健康检查端点 |
| test_models_list | 模型列表 API |
| test_non_streaming_basic | 非流式基本功能 |
| test_non_streaming_metrics | 非流式指标验证 |
| test_streaming_basic | 流式输出验证 |
| test_streaming_ttft | TTFT 首 Token 时延测量 |
| test_repeated_requests_stability | 重复请求稳定性 |
| test_input_length_scaling | 输入长度扩展测试 (低/中/高) |

## 系统要求

| 配置项 | 最低要求 | 推荐配置 |
|--------|----------|----------|
| CPU | 4核 | 8核+ |
| 内存 | 4GB | 8GB+ (Q4) / 16GB+ (Q8) |
| 存储 | 3GB | 5GB+ |

## 文档

- [架构设计文档](docs/架构设计文档.md)
- [GGUF 量化说明](docs/gguf-quantization.md)
- [性能质量指标体系](docs/大模型性能质量指标体系——详细解释.pdf)

## License

MIT