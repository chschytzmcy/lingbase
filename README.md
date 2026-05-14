# Lingbase

边缘 LLM 推理服务 - 基于 llama.cpp 的 OpenAI 兼容 API 服务。

## 特性

- **OpenAI 兼容**: 支持 `/v1/chat/completions` API
- **流式输出**: SSE 实时 token 流式返回
- **跨平台**: 支持 x86/ARM CPU
- **轻量高效**: 基于 llama.cpp，无重型依赖

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

# 或下载其他量化版本
modelscope download --model Qwen/Qwen3-4B-GGUF --include "Qwen3-4B-Q8_0.gguf"
```

模型下载路径默认为：
```
~/.cache/modelscope/hub/models/Qwen/Qwen3-4B-GGUF/
```

#### 从 HuggingFace 下载

```bash
# 安装 huggingface-cli
pip install huggingface_hub

# 下载模型
huggingface-cli download Qwen/Qwen3-4B-GGUF Qwen3-4B-Q4_K_M.gguf --local-dir ./models
```

#### GGUF 量化模型对比

| 文件名 | 大小 | 适用场景 |
|--------|------|----------|
| Qwen3-4B-Q4_K_M.gguf | 2.4G | 边缘设备，内存受限 |
| Qwen3-4B-Q5_K_M.gguf | 2.7G | 平衡速度与质量 (推荐) |
| Qwen3-4B-Q6_K.gguf | 3.1G | 高质量推理 |
| Qwen3-4B-Q8_0.gguf | 4.0G | 接近原始精度 |

详见 [GGUF 量化说明](docs/gguf-quantization.md)。

### 3. 配置模型路径

修改 `config/environment.toml`：

```toml
[model]
test_model_path = "/home/etsme/.cache/modelscope/hub/models/Qwen/Qwen3-4B-GGUF/Qwen3-4B-Q4_K_M.gguf"
context_size = 8192
max_prompt_tokens = 8192
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
curl -X POST http://localhost:11017/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "Qwen3-4B",
    "messages": [{"role": "user", "content": "你好，介绍一下自己"}],
    "max_tokens": 128
  }'

# 流式请求
curl -X POST http://localhost:11017/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "Qwen3-4B",
    "messages": [{"role": "user", "content": "你好，介绍一下自己"}],
    "stream": true,
    "max_tokens": 128
  }'

# 健康检查
curl http://localhost:11017/health
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

### GET /health

健康检查接口。

## 项目结构

```
lingbase/
├── src/
│   ├── api/           # HTTP API 层
│   ├── backend/       # 推理后端抽象
│   ├── llama/         # llama.cpp FFI 封装
│   └── infra/         # 基础设施 (配置、日志、健康检查)
├── config/            # 配置文件
├── lib/               # 预编译 llama.cpp 库
└── docs/              # 设计文档
```

## 系统要求

| 配置项 | 最低要求 | 推荐配置 |
|--------|----------|----------|
| CPU | 4核 | 8核+ |
| 内存 | 4GB | 8GB+ (Q4) / 16GB+ (Q8) |
| 存储 | 3GB | 5GB+ |

## 文档

- [架构设计文档](docs/架构设计文档.md)
- [GGUF 量化说明](docs/gguf-quantization.md)

## License

MIT