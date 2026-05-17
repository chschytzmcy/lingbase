# rknn3-sys

Rockchip RKNN3 Runtime 的安全 Rust 绑定。封装了 `librknn3_api` 的全部 FFI 调用，提供纯安全 API。

## 平台

仅支持 **aarch64**（RK3588、RK1828 等 Rockchip NPU 设备）。交叉编译：

```bash
cargo zigbuild --target aarch64-unknown-linux-gnu --release
```

## 快速开始

```rust
use rknn3_sys::prelude::*;

// 1. 初始化 Runtime
let ctx = Context::new()?;

// 2. 加载模型
ctx.load_model("model.rknn", "model.weight")?;

// 3. 配置并初始化模型
let mut config = ModelConfig::new().core_mask(0xff);
ctx.model_init(&mut config)?;

// 4. 创建 LLM Session
let llm_param = LlmParams::new("logits", 151936)?
    .max_context_len(4096)
    .top_k(40)
    .top_p(0.9)
    .temperature(0.8);

let mut session = Session::new(&ctx, &mut [llm_param])?;

// 5. 设置回调（tokenizer + embedding + 输出处理）
session.set_callback(Box::new(my_callbacks))?;

// 6. 运行推理
let mut input = LlmInput::prompt("Hello").role("user")?;
let mut params = InferParams::new().max_new_tokens(512);
session.run(&mut [input], &mut params)?;
```

## 核心 API

### Context — Runtime 上下文

| 方法 | 说明 |
|------|------|
| `Context::new()` | 初始化 RKNN3 Runtime |
| `ctx.load_model(model, weight)` | 从文件加载模型 |
| `ctx.load_model_from_data(model, weight)` | 从内存加载模型 |
| `ctx.model_init(&mut config)` | 初始化已加载的模型 |
| `ctx.dup_context()` | 复制上下文（用于并发） |
| `ctx.find_devices()` | 查询可用 NPU 设备 |

### Session — LLM 推理会话

| 方法 | 说明 |
|------|------|
| `Session::new(ctx, params)` | 创建 LLM 会话 |
| `session.set_callback(cb)` | 设置回调处理器 |
| `session.run(inputs, params)` | 同步推理 |
| `session.run_async(inputs, params)` | 异步推理 |
| `session.stop()` | 停止推理 |
| `session.query_state()` | 查询推理状态 |
| `session.set_chat_template(...)` | 设置聊天模板 |
| `session.set_function_tools(json)` | 设置 Function Calling 工具 |
| `session.set_kvcache_policy(...)` | 设置 KV Cache 策略 |
| `session.clear_kvcache(...)` | 清除 KV Cache |
| `session.enable_lora(...)` / `disable_lora(...)` | LoRA 适配器管理 |

### LlmCallbacks — 回调 Trait

```rust
pub trait LlmCallbacks {
    fn tokenize(&self, text: &str, buf: &mut [i32]) -> Result<usize, CallbackError>;
    fn embed(&self, tokens: &[i32], buf: &mut [u8]) -> Result<(), CallbackError>;
    fn on_result(&mut self, token_ids: &[i32], state: LlmCallState);
}
```

- `tokenize` — 文本分词，由 Runtime 在需要时调用
- `embed` — 查找 token 对应的嵌入向量
- `on_result` — 推理结果回调（逐 token），`LlmCallState` 标识状态

### LlmInput — 推理输入

```rust
// 文本输入（Runtime 内部分词）
LlmInput::prompt("你好").role("user")?

// Token 输入（外部预分词）
LlmInput::tokens(vec![1, 2, 3]).role("user")?

// 启用思考模式
LlmInput::prompt("思考一下").role("user")?.enable_thinking(true)
```

### 通用推理 API

除 LLM 专用接口外，也支持通用模型推理：

```rust
let mut input = Tensor::new();
let mut output = Tensor::new();
ctx.run(&[input], &mut [output])?;
```

## 模型文件

LLM 推理需要以下文件（位于同一目录）：

| 文件 | 说明 |
|------|------|
| `model.rknn` | RKNN3 模型文件 |
| `model.weight` | 模型权重 |
| `model.tokenizer.gguf` | GGUF 格式 tokenizer |
| `model.embed.bin` | 嵌入表（float16，vocab_size × dim × 2 字节） |

## CLI 示例

```bash
# 基础推理
cargo run --example rknn3-cli -- \
  --model model.rknn --weight model.weight \
  --tokenizer model.tokenizer.gguf --embedding model.embed.bin \
  --prompt "Hello"

# 思考模式
cargo run --example rknn3-cli -- \
  --model model.rknn --weight model.weight \
  --tokenizer model.tokenizer.gguf --embedding model.embed.bin \
  --prompt "Explain quantum computing" --think
```

## 错误处理

所有 FFI 调用返回 `Result<T, Error>`，错误类型涵盖 RKNN3 全部错误码：

```rust
match ctx.load_model("model.rknn", "model.weight") {
    Ok(()) => {},
    Err(Error::ApiFail { function }) => eprintln!("{function} failed"),
    Err(Error::ApiTimeout { function }) => eprintln!("{function} timed out"),
    Err(Error::ApiModelInvalid { function }) => eprintln!("invalid model in {function}"),
    Err(e) => eprintln!("other error: {e}"),
}
```

## 文档

Rockchip 官方 SDK 文档位于 `docs/` 目录。
