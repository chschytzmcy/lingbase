# Lingbase RKNN3 后端设计方案

## 1. 背景

Lingbase 当前支持 CPU/CUDA/RKLLM 后端，etsllm 项目已有成熟的 rknn3-sys 绑定和 runner-rknn3 实现。本方案参考 etsllm 经验，为 lingbase 增加 RKNN3 后端支持。

## 2. 依赖引入

```toml
# Cargo.toml
[dependencies]
rknn3-sys = { path = "path/to/etsllm/crates/rknn3-sys" }

[build-dependencies]
cmake = "0.1"
cc = "1.0"
```

## 3. BackendType 扩展

```rust
// src/backend/factory.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Cuda,
    Cpu,
    Rkllm,
    Rknn3,  // 新增
}
```

## 4. Rknn3Backend 实现

新建 `src/backend/rknn3.rs`：

```rust
use rknn3_sys::prelude::*;

pub struct Rknn3Backend {
    context: Context,
    session: Session,
    options: Rknn3Options,
    tokenizer: shimmytok::Tokenizer,
    embedding: Embedding,
    n_vocab: usize,
    n_ctx: usize,
}

pub struct Rknn3Options {
    pub max_concurrency: usize,
    pub context_size: usize,
    pub temperature: f32,
    pub top_k: usize,
    pub top_p: f32,
    pub vocab_size: i32,
    pub logits_name: String,
    pub repeat_penalty: f32,
    pub special_bos_id: Vec<i32>,
    pub special_eos_id: Vec<i32>,
    pub skip_special_token: bool,
    pub core_mask: String,
}
```

## 5. InferenceBackend trait 实现

```rust
impl InferenceBackend for Rknn3Backend {
    fn name(&self) -> &str { "rknn3" }

    fn forward(&self, tokens: &[i32], config: &InferenceConfig) -> InferenceResult<ForwardResult> {
        // 非流式推理：收集所有 token 后一次性返回
    }

    fn forward_stream(&self, tokens: &[i32], config: &InferenceConfig)
        -> Pin<Box<dyn Stream<Item = InferenceResult<StreamToken>> + Send>> {
        // 流式推理：通过 LlmCallbacks 回调逐 token 发送
        // 参考 runner-rknn3/src/lib.rs 的 InferenceCallbacks 实现
    }

    fn sample_token(&self, logits: &[f32], config: &InferenceConfig) -> i32 {
        // rknn3 不暴露原始 logits，由 Session 内部采样
        // 需返回 token_id（调用方无法自定义采样）
    }

    fn tokenize(&self, text: &str) -> InferenceResult<Vec<i32>> { ... }
    fn detokenize(&self, tokens: &[i32]) -> InferenceResult<String> { ... }
}
```

## 6. 初始化流程

```rust
impl Rknn3Backend {
    pub fn new(model_path: &Path, n_ctx: i32) -> InferenceResult<Self> {
        // 1. 创建 Context
        let ctx = Context::new()?;

        // 2. 加载模型
        ctx.load_model(
            model_path.join("model.rknn"),
            model_path.join("model.weight"),
        )?;

        // 3. model_init
        let mut config = ModelConfig::new().core_mask(core_mask);
        ctx.model_init(&mut config)?;

        // 4. 创建 Session
        let llm_params = LlmParams::new("logits", vocab_size)
            .max_context_len(n_ctx as i32)
            .temperature(temperature)
            // ...
        let session = Session::new(&ctx, &mut [llm_params])?;

        // 5. 加载分词器和 embedding
        let tokenizer = Tokenizer::from_gguf_file(&model_path.join("model.tokenizer.gguf"))?;
        let embedding = Embedding::from_file(&model_path.join("model.embed.bin"), vocab_size)?;

        Ok(Self { context, session, ... })
    }
}
```

## 7. 与现有实现的差异

| 维度 | lingbase rknn3 | etsllm runner-rknn3 |
|------|----------------|---------------------|
| 流式输出 | Sse via mpsc channel | DroppableReceiver |
| 回调 | InferenceCallbacks 内部发送 channel | 同左 |
| 池化 | 单 Session | deadpool::Pool 多实例 dup_context |
| 采样 | rknn3 内部采样 | 同左 |
| Embedding | 外部加载 | 同左 |

## 8. 文件变更清单

```
src/backend/
├── mod.rs           # 添加 rknn3 模块导出
├── factory.rs       # BackendType::Rknn3 + create 分支
└── rknn3.rs         # 新建：Rknn3Backend 实现

Cargo.toml           # 添加 rknn3-sys 依赖
build.rs             # 可选：添加 rknn3 库的 cmake 构建
```

## 9. 风险点

1. **sample_token 不暴露 logits**：rknn3 Session 内部完成采样，`InferenceBackend::sample_token` 无法被正确实现（返回固定 token 或 error）

2. **模型格式差异**：需确认 lingbase 使用的模型是否已导出为 rknn3 格式（.rknn + .weight）

3. **依赖传递**：rknn3-sys 依赖 build-tools 的 PrebuiltLibBuilder，需要确保路径正确

## 10. 参考实现

- etsllm/crates/rknn3-sys：RKNN3 safe bindings
- etsllm/runner-rknn3/src/lib.rs：完整的 runner-rknn3 实现，包含池化、流式输出、UTF-8 跨 token 边界处理