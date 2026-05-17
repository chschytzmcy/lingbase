//! CPU 硬件后端实现

use std::path::Path;
use async_trait::async_trait;

use crate::error::{InferenceError, InferenceResult};
use crate::llama::{LlamaModel, LlamaContext, Tokenizer};
use crate::backend::runner::llama_hardware::{LlamaHardware, LlamaModelHandle, LlamaRunner};
use crate::backend::traits::{BaseBackend, LLMBackend, StreamChunk, DroppableReceiver, LLMInferenceInput, LLMInferenceOptions};
use crate::backend::error::BackendError;
use crate::backend::types::ResourceType;
use crate::backend::runner::sample::sample_token;
use crate::backend::{InferenceBackend, InferenceConfig, ForwardResult, StreamToken, MemoryStats};

/// CpuHardware - CPU 硬件后端
pub struct CpuHardware;

impl CpuHardware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CpuHardware {
    fn default() -> Self {
        Self::new()
    }
}

impl LlamaHardware for CpuHardware {
    fn name(&self) -> &'static str {
        "llama-cpu"
    }

    fn load_model(&self, model_path: &Path, _n_ctx: i32) -> InferenceResult<LlamaModelHandle> {
        let model = LlamaModel::from_file(model_path, 0)?;
        let vocab_ptr = model.vocab_ptr().ok_or(InferenceError::BackendNotInitialized)?;

        Ok(LlamaModelHandle {
            model,
            vocab_ptr: Some(vocab_ptr),
        })
    }

    fn load_tokenizer(&self, model: &LlamaModel) -> InferenceResult<Tokenizer> {
        let vocab_ptr = model.vocab_ptr().ok_or(InferenceError::BackendNotInitialized)?;
        Ok(Tokenizer::new(vocab_ptr))
    }

    fn vocab_size(&self, model: &LlamaModel) -> usize {
        model.n_vocab() as usize
    }

    fn load_context(&self, model_ptr: Option<crate::llama::ffi::ModelPtr>, n_ctx: u32, n_threads: i32) -> InferenceResult<LlamaContext> {
        let ptr = model_ptr.ok_or(InferenceError::ModelNotLoaded)?;
        LlamaContext::new(ptr, n_ctx, n_threads as u32)
    }
}

/// CpuBackend - CPU 后端实现
pub struct CpuBackend {
    runner: LlamaRunner<CpuHardware>,
}

impl CpuBackend {
    pub fn new<P: AsRef<Path>>(model_path: P, n_ctx: i32) -> InferenceResult<Self> {
        let mut runner = LlamaRunner::new(CpuHardware::new());
        runner.init(model_path.as_ref(), n_ctx as u32, 4)?;
        Ok(Self { runner })
    }

    pub fn is_initialized(&self) -> bool {
        self.runner.health_check()
    }
}

// 实现 BaseBackend + LLMBackend 以便在 BackendManager 中使用
#[async_trait]
impl BaseBackend for CpuBackend {
    async fn init(_model_config: &crate::infra::config::ModelConfig) -> Result<Self, BackendError> {
        Err(BackendError::BackendNotInitialized)
    }

    fn name(&self) -> &'static str {
        "llama-cpu"
    }

    fn health_check(&self) -> bool {
        self.is_initialized()
    }

    fn resource_usage(&self) -> Vec<ResourceType> {
        vec![]
    }
}

#[async_trait]
impl LLMBackend for CpuBackend {
    async fn inference_stream(
        &self,
        input: LLMInferenceInput,
        options: LLMInferenceOptions,
    ) -> Result<DroppableReceiver<StreamChunk>, BackendError> {
        self.runner.inference_stream(input, options).await
    }

    async fn context_size(&self) -> Result<usize, BackendError> {
        self.runner.context_size().await
    }
}

// 保留向后兼容的 InferenceBackend 实现
impl InferenceBackend for CpuBackend {
    fn name(&self) -> &str {
        "cpu"
    }

    fn health_check(&self) -> bool {
        self.is_initialized()
    }

    fn memory_stats(&self) -> MemoryStats {
        MemoryStats {
            used_bytes: 0,
            total_bytes: 0,
            free_bytes: 0,
        }
    }

    fn forward(&self, tokens: &[i32], config: &InferenceConfig) -> InferenceResult<ForwardResult> {
        let start = std::time::Instant::now();
        let tokens: Vec<u32> = tokens.iter().map(|&t| t as u32).collect();
        let input = LLMInferenceInput::Tokens(tokens);
        let options = LLMInferenceOptions {
            id: uuid::Uuid::new_v4().to_string(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            top_p: config.top_p,
            top_k: config.top_k as usize,
            repeat_penalty: config.repeat_penalty,
            stream: false,
            stop: vec![],
            enable_thinking: None,
        };

        let rt = tokio::runtime::Runtime::new().map_err(|e| InferenceError::InferenceFailed(e.to_string()))?;
        let mut receiver = rt.block_on(self.runner.inference_stream(input, options))?;

        let all_tokens = Vec::new();
        let mut first_token_ms = None;

        use futures::StreamExt;
        while let Some(chunk) = rt.block_on(receiver.into_stream().next()) {
            match chunk {
                StreamChunk::Normal(crate::backend::traits::LLMSteamChunkType::RawToken(_)) => {
                    if first_token_ms.is_none() {
                        first_token_ms = Some(start.elapsed().as_millis() as u64);
                    }
                }
                StreamChunk::Finish(_) => break,
                StreamChunk::Error(e) => {
                    return Err(InferenceError::InferenceFailed(e));
                }
                _ => {}
            }
        }

        Ok(ForwardResult {
            tokens: all_tokens,
            first_token_ms,
            total_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn forward_stream(
        &self,
        tokens: &[i32],
        config: &InferenceConfig,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = InferenceResult<StreamToken>> + Send>> {
        let tokens: Vec<u32> = tokens.iter().map(|&t| t as u32).collect();
        let input = LLMInferenceInput::Tokens(tokens);
        let options = LLMInferenceOptions {
            id: uuid::Uuid::new_v4().to_string(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            top_p: config.top_p,
            top_k: config.top_k as usize,
            repeat_penalty: config.repeat_penalty,
            stream: true,
            stop: vec![],
            enable_thinking: None,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut receiver = rt.block_on(self.runner.inference_stream(input, options)).unwrap();

        use futures::StreamExt;
        receiver.into_stream()
            .map(|chunk| {
                match chunk {
                    StreamChunk::Normal(crate::backend::traits::LLMSteamChunkType::RawToken(text)) => Ok(StreamToken {
                        token: 0,
                        text,
                        is_first: false,
                        is_done: false,
                    }),
                    StreamChunk::Finish(_) => Ok(StreamToken {
                        token: 0,
                        text: String::new(),
                        is_first: false,
                        is_done: true,
                    }),
                    StreamChunk::Error(e) => {
                        Err(InferenceError::InferenceFailed(e))
                    }
                    _ => Ok(StreamToken {
                        token: 0,
                        text: String::new(),
                        is_first: false,
                        is_done: false,
                    }),
                }
            })
            .boxed()
    }

    fn max_context_size(&self) -> usize {
        tokio::runtime::Handle::current().block_on(async {
            self.runner.context_size().await.unwrap_or(0)
        })
    }

    fn sample_token(&self, logits: &[f32], config: &InferenceConfig) -> i32 {
        sample_token(logits, config.temperature, config.top_p, config.top_k as usize, 0) as i32
    }

    fn tokenize(&self, _text: &str) -> InferenceResult<Vec<i32>> {
        Ok(vec![])
    }

    fn detokenize(&self, tokens: &[i32]) -> InferenceResult<String> {
        Ok(tokens.iter().map(|t| t.to_string()).collect())
    }
}