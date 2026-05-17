# Lingbase

边缘 LLM 推理服务 - 基于 llama.cpp 的 OpenAI 兼容 API 服务。

## 特性

- **OpenAI 兼容**: 支持 `/v1/chat/completions` API
- **流式输出**: SSE 实时 token 流式返回
- **异构硬件**: 支持 CPU/CUDA/RKNN3 后端抽象
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
[server]
host = "0.0.0.0"
port = 11017
workers = 4

[model]
name = "qwen3-4b"
model_path = "/path/to/Qwen3-4B-Q4_K_M.gguf"
context_size = 16384
auto_load = true

[model.inference]
max_tokens = 256
temperature = 0.7
top_p = 0.9
top_k = 40
repeat_penalty = 1.1

[model.backend]
backend_type = "llama-cpu"
```

### 4. 编译运行

```bash
# x86_64 编译
cargo build --release

# aarch64 交叉编译
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  cargo build --release --target aarch64-unknown-linux-gnu

# 运行
./scripts/run.sh
```

### 5. 测试 API

```bash
# 健康检查
curl http://localhost:11017/health

# 模型列表
curl http://localhost:11017/v1/models

# 非流式请求
curl -X POST http://localhost:11017/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3-4b",
    "messages": [{"role": "user", "content": "你好"}],
    "max_tokens": 128
  }'

# 流式请求
curl -X POST http://localhost:11017/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3-4b",
    "messages": [{"role": "user", "content": "你好"}],
    "stream": true,
    "max_tokens": 128
  }'
```

## 架构

Lingbase 采用三层 trait 抽象实现异构硬件解耦：

```
traits.rs (BaseBackend + LLMBackend)
    ↓
runner/llama_hardware.rs (LlamaHardware + LlamaRunner<H>)
    ↓
cpu.rs (CpuHardware) / runner_rknn3.rs (Rknn3Backend)
```

- **BaseBackend**: 所有后端的公共接口（初始化、资源）
- **LLMBackend**: LLM 推理能力（流式推理、上下文大小）
- **LlamaHardware**: llama.cpp 硬件抽象（模型加载、分词器）
- **LlamaRunner<H>**: 通用推理 Runner，通过泛型 H 抽象硬件差异

详见 [架构设计文档](docs/architecture-design.md)

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
│   ├── main.rs                     # 入口
│   ├── lib.rs                      # 库导出
│   ├── api/                        # HTTP API 层 (Axum)
│   ├── backend/                    # 核心后端抽象
│   │   ├── traits.rs               # BaseBackend + LLMBackend
│   │   ├── types.rs                # BackendType, FinishReason
│   │   ├── error.rs                # BackendError
│   │   ├── cpu.rs                  # CpuBackend（实现三层 trait）
│   │   ├── manager.rs              # BackendManager
│   │   └── runner/
│   │       ├── llama_hardware.rs   # LlamaHardware + LlamaRunner
│   │       └── sample.rs           # 采样算法
│   └── infra/                      # 基础设施
│       └── config.rs               # 配置结构
├── lib/                            # 预编译 llama.cpp 库
│   ├── aarch64/                    # ARM64 库
│   ├── x86_64-cpu/                 # x86_64 CPU 库
│   └── rknn3-aarch64/              # RKNN3 预编译库
├── config/                         # 配置文件
├── tests/                          # 集成测试 (Python)
└── docs/                           # 设计文档
```

## 测试

### 运行测试

```bash
# 安装依赖
pip install requests pytest --break-system-packages

# 运行测试
./tests/run_tests.sh
```

## 文档

- [架构设计文档](docs/architecture-design.md) - 详细的架构和设计说明
- [RKNN3 后端设计](docs/rknn3-backend-design.md) - RKNN3 硬件后端实现
- [GGUF 量化说明](docs/gguf-quantization.md) - 模型量化格式说明

## License

MIT