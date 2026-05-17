//! LlamaHardware trait - 抽象 llama.cpp 硬件后端差异
//!
//! 通过 trait 抽象 CPU/CUDA 等不同硬件的模型加载方式，
//! 使得 LlamaRunner<H> 可以复用相同的推理逻辑。

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use crate::error::InferenceResult;
use crate::infra::config::ModelConfig;
use crate::llama::{LlamaModel, LlamaContext, Tokenizer, batch::BatchWithTokens};
use crate::backend::types::FinishReason;
use crate::backend::traits::{BaseBackend, LLMBackend, LLMInferenceInput, LLMInferenceOptions, DroppableReceiver, StreamChunk, LLMSteamChunkType};
use crate::backend::error::BackendError;
use super::sample::sample_token;

/// Llama 硬件后端特征
///
/// 定义不同硬件（CPU/CUDA）如何加载模型和解码
pub trait LlamaHardware: Send + Sync {
    /// 硬件名称
    fn name(&self) -> &'static str;

    /// 加载 Llama 模型
    fn load_model(&self, model_path: &Path, n_ctx: i32) -> InferenceResult<LlamaModelHandle>;

    /// 加载分词器
    fn load_tokenizer(&self, model: &LlamaModel) -> InferenceResult<Tokenizer>;

    /// 获取 vocab 大小
    fn vocab_size(&self, model: &LlamaModel) -> usize;

    /// 加载上下文
    fn load_context(&self, model_ptr: Option<crate::llama::ffi::ModelPtr>, n_ctx: u32, n_threads: i32) -> InferenceResult<LlamaContext>;
}

/// Llama 模型句柄
pub struct LlamaModelHandle {
    pub model: LlamaModel,
    pub vocab_ptr: Option<crate::llama::ffi::VocabPtr>,
}

unsafe impl Send for LlamaModelHandle {}
unsafe impl Sync for LlamaModelHandle {}

impl LlamaModelHandle {
    pub fn ptr(&self) -> Option<crate::llama::ffi::ModelPtr> {
        self.model.ptr()
    }

    pub fn vocab_ptr(&self) -> Option<crate::llama::ffi::VocabPtr> {
        self.vocab_ptr
    }
}

/// LlamaRunner - 通用的 llama.cpp 推理_runner
///
/// 通过泛型 H 抽象硬件差异，复用相同的推理逻辑
pub struct LlamaRunner<H: LlamaHardware> {
    hardware: H,
    model_handle: Option<LlamaModelHandle>,
    context: Arc<tokio::sync::Mutex<Option<LlamaContext>>>,
    tokenizer: Arc<tokio::sync::Mutex<Option<Tokenizer>>>,
    n_ctx: u32,
    n_vocab: usize,
}

impl<H: LlamaHardware> LlamaRunner<H> {
    pub fn new(hardware: H) -> Self {
        Self {
            hardware,
            model_handle: None,
            context: Arc::new(tokio::sync::Mutex::new(None)),
            tokenizer: Arc::new(tokio::sync::Mutex::new(None)),
            n_ctx: 0,
            n_vocab: 0,
        }
    }

    /// 初始化模型
    pub fn init(&mut self, model_path: &Path, n_ctx: u32, n_threads: i32) -> InferenceResult<()> {
        let model_handle = self.hardware.load_model(model_path, n_ctx as i32)?;
        let n_vocab = self.hardware.vocab_size(&model_handle.model);

        let tokenizer = self.hardware.load_tokenizer(&model_handle.model)?;
        let context = self.hardware.load_context(model_handle.ptr(), n_ctx, n_threads)?;

        self.model_handle = Some(model_handle);
        self.n_ctx = n_ctx;
        self.n_vocab = n_vocab;

        // 设置 tokenizer 和 context
        let mut tok_lock = self.tokenizer.try_lock().unwrap();
        *tok_lock = Some(tokenizer);
        drop(tok_lock);

        let mut ctx_lock = self.context.try_lock().unwrap();
        *ctx_lock = Some(context);
        drop(ctx_lock);

        Ok(())
    }
}

#[async_trait]
impl<H: LlamaHardware> BaseBackend for LlamaRunner<H> {
    async fn init(model_config: &ModelConfig) -> Result<Self, BackendError> {
        // LlamaRunner 不通过此方法初始化，需要通过 init() 方法
        let _ = model_config;
        Err(BackendError::BackendNotInitialized)
    }

    fn name(&self) -> &'static str {
        self.hardware.name()
    }

    fn health_check(&self) -> bool {
        self.model_handle.is_some()
    }

    fn resource_usage(&self) -> Vec<crate::backend::types::ResourceType> {
        vec![]
    }
}

#[async_trait]
impl<H: LlamaHardware + 'static> LLMBackend for LlamaRunner<H> {
    async fn inference_stream(
        &self,
        input: LLMInferenceInput,
        options: LLMInferenceOptions,
    ) -> Result<DroppableReceiver<StreamChunk>, BackendError> {
        // 转换输入
        let tokens: Vec<i32> = match input {
            LLMInferenceInput::Tokens(tokens) => tokens.iter().map(|&t| t as i32).collect(),
            LLMInferenceInput::Messages(messages) => {
                let text = messages
                    .iter()
                    .map(|m| format!("<|im_start|>{}\n{}<|im_end|>", m.role, m.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                let text = text + "<|im_start|>assistant\n";
                self.tokenize(&text).map_err(|e| BackendError::TokenizerFailed(e.to_string()))?
                    .iter().map(|&t| t as i32).collect()
            }
        };

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();

        let context = Arc::clone(&self.context);
        let tokenizer = Arc::clone(&self.tokenizer);
        let n_vocab = self.n_vocab;
        let config = options.clone();

        // 获取 context 和 tokenizer
        let ctx_guard = context.lock().await;
        let ctx = ctx_guard.as_ref().ok_or(BackendError::BackendNotInitialized)?;

        let tok_guard = tokenizer.lock().await;
        let tok = tok_guard.as_ref().ok_or(BackendError::BackendNotInitialized)?;

        // Prefill phase
        let batch = BatchWithTokens::new(&tokens)
            .map_err(|e| BackendError::InferenceFailed(e.to_string()))?;

        ctx.decode(batch.batch)
            .map_err(|e| BackendError::InferenceFailed(e.to_string()))?;

        // Get logits and sample first token
        let logits = ctx.get_logits(n_vocab)
            .ok_or_else(|| BackendError::InferenceFailed("Failed to get logits".to_string()))?;

        let _first_token_ms = std::time::Instant::now().elapsed().as_millis() as u64;
        let sampled_token = sample_token(logits, config.temperature, config.top_p, config.top_k, n_vocab) as i32;

        // Send first token
        let text = tok.decode_tokens(&[sampled_token], true)
            .map_err(|e| BackendError::TokenizerFailed(e.to_string()))?;

        sender.send(StreamChunk::Normal(LLMSteamChunkType::RawToken(text)))
            .map_err(|_| BackendError::Internal("Channel closed".into()))?;

        // Generation loop - 在 spawn_blocking 中运行
        drop(ctx_guard);
        drop(tok_guard);

        let context_inner = Arc::clone(&context);
        let tokenizer_inner = Arc::clone(&tokenizer);
        let sender_inner = sender.clone();

        let _handle = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let ctx_guard = context_inner.lock().await;
                let ctx = ctx_guard.as_ref().unwrap();

                let tok_guard = tokenizer_inner.lock().await;
                let tok = tok_guard.as_ref().unwrap();

                let mut prev_token = sampled_token;

                for _ in 1..config.max_tokens {
                    // Check EOS
                    if tok.is_eog(prev_token) {
                        let _ = sender_inner.send(StreamChunk::Finish(FinishReason::Stop));
                        return;
                    }

                    // Decode next token
                    let batch = match BatchWithTokens::new(&[prev_token]) {
                        Ok(b) => b,
                        Err(e) => {
                            let _ = sender_inner.send(StreamChunk::Error(e.to_string()));
                            return;
                        }
                    };

                    if let Err(e) = ctx.decode(batch.batch) {
                        let _ = sender_inner.send(StreamChunk::Error(e.to_string()));
                        return;
                    }

                    let logits = match ctx.get_logits(n_vocab) {
                        Some(l) => l,
                        None => {
                            let _ = sender_inner.send(StreamChunk::Error("Failed to get logits".into()));
                            return;
                        }
                    };

                    let next_token = sample_token(logits, config.temperature, config.top_p, config.top_k, n_vocab) as i32;

                    // Send token
                    let text = match tok.decode_tokens(&[next_token], true) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };

                    let _ = sender_inner.send(StreamChunk::Normal(LLMSteamChunkType::RawToken(text)));

                    prev_token = next_token;
                }

                // Max tokens reached
                let _ = sender_inner.send(StreamChunk::Finish(FinishReason::Length));
            });
        });

        Ok(DroppableReceiver::new(
            receiver,
            Some(Box::new(move || {
                // abort 逻辑
                Ok(())
            })),
        ))
    }

    async fn context_size(&self) -> Result<usize, BackendError> {
        Ok(self.n_ctx as usize)
    }
}

impl<H: LlamaHardware> LlamaRunner<H> {
    fn tokenize(&self, _text: &str) -> InferenceResult<Vec<u32>> {
        // 简化实现，实际需要 tokenizer
        Ok(vec![])
    }
}