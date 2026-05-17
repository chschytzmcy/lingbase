//! RKNN3 后端实现
//!
//! 基于 aarch64-rknn 提供 LLM 推理能力。

use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};

use async_trait::async_trait;
use deadpool::unmanaged::{Pool as UnmanagedPool, PoolConfig};
use deadpool::Runtime as DeadpoolRuntime;
use tokio::time::Duration;
use tracing::{debug, error, trace};

use crate::infra::config::ModelConfig;
use super::types::{FinishReason, ResourceType};
use super::traits::{BaseBackend, LLMBackend, LLMInferenceInput, LLMInferenceOptions, DroppableReceiver, StreamChunk, LLMSteamChunkType};
use super::error::BackendError;

/// RKNN3 配置选项
#[derive(Debug, Clone)]
pub struct Rknn3Options {
    pub max_concurrency: usize,
    pub pool_timeout_secs: u64,
    pub context_size: usize,
    pub temperature: f32,
    pub top_k: usize,
    pub top_p: f32,
    pub vocab_size: i32,
    pub logits_name: String,
    pub repeat_penalty: f32,
    pub special_bos_id: Vec<i32>,
    pub special_eos_id: Vec<i32>,
    pub linefeed_id: i32,
    pub skip_special_token: bool,
    pub core_mask: String,
}

impl Default for Rknn3Options {
    fn default() -> Self {
        Self {
            max_concurrency: 1,
            pool_timeout_secs: 5,
            context_size: 4096,
            temperature: 0.7,
            top_k: 40,
            top_p: 0.9,
            vocab_size: 151936,
            logits_name: "logits".to_string(),
            repeat_penalty: 1.2,
            special_bos_id: vec![151643],
            special_eos_id: vec![151643, 151645, 151662, 151663, 151664],
            linefeed_id: 198,
            skip_special_token: true,
            core_mask: "0xff".to_string(),
        }
    }
}

/// RKNN3 错误类型
#[derive(Debug, thiserror::Error)]
pub enum Rknn3Error {
    #[error("Model init failed: {0}")]
    ModelInitFailed(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("LLM is busy: {0}")]
    LlmBusy(String),

    #[error("Invalid model path: {0}")]
    InvalidModelPath(String),

    #[error("Tokenizer failed: {0}")]
    TokenizerFailed(String),

    #[error("Embedding failed: {0}")]
    EmbeddingFailed(String),

    #[error("Inference error: {0}")]
    InferenceError(String),
}

impl From<Rknn3Error> for BackendError {
    fn from(e: Rknn3Error) -> Self {
        match e {
            Rknn3Error::ModelInitFailed(s) => BackendError::InferenceFailed(s),
            Rknn3Error::SessionError(s) => BackendError::InferenceFailed(s),
            Rknn3Error::LlmBusy(s) => BackendError::LLMBusy(s),
            Rknn3Error::InvalidModelPath(s) => BackendError::Internal(s),
            Rknn3Error::TokenizerFailed(s) => BackendError::TokenizerFailed(s),
            Rknn3Error::EmbeddingFailed(s) => BackendError::Internal(s),
            Rknn3Error::InferenceError(s) => BackendError::InferenceFailed(s),
        }
    }
}

// ---------------------------------------------------------------------------
// Byte-buffered decoder for streaming token output
// ---------------------------------------------------------------------------
//
// RKNN3 calls on_result per-token. Emoji UTF-8 bytes are split across multiple
// tokens. We accumulate raw bytes and flush complete UTF-8 characters.

struct ByteBufferedDecoder {
    buf: Vec<u8>,
}

impl ByteBufferedDecoder {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn piece_to_raw_bytes(piece: &str) -> Vec<u8> {
        let decoder = shimmytok::byte_encoder::unicode_to_bytes();
        piece
            .chars()
            .filter_map(|c| decoder.get(&c).copied())
            .collect()
    }

    fn feed(&mut self, pieces: &[&str]) -> String {
        for piece in pieces {
            let raw = Self::piece_to_raw_bytes(piece);
            self.buf.extend_from_slice(&raw);
        }
        self.drain_utf8()
    }

    fn flush(&mut self) -> String {
        let result = String::from_utf8_lossy(&self.buf).into_owned();
        self.buf.clear();
        result
    }

    fn drain_utf8(&mut self) -> String {
        match std::str::from_utf8(&self.buf) {
            Ok(_) => {
                let s = std::mem::take(&mut self.buf);
                String::from_utf8(s).unwrap_or_default()
            }
            Err(e) => {
                let valid_up_to = e.valid_up_to();
                if valid_up_to == 0 {
                    let keep = self.buf.len().min(3);
                    let drain = self.buf.len() - keep;
                    let valid_bytes: Vec<u8> = self.buf[..drain].to_vec();
                    self.buf.drain(..drain);
                    String::from_utf8_lossy(&valid_bytes).into_owned()
                } else {
                    let valid_bytes: Vec<u8> = self.buf[..valid_up_to].to_vec();
                    self.buf.drain(..valid_up_to);
                    String::from_utf8(valid_bytes).unwrap_or_default()
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Embedding (float16 lookup table)
// ---------------------------------------------------------------------------

struct Embedding {
    data: Vec<u8>,
    vocab_size: usize,
    embedding_dim: usize,
}

impl Embedding {
    fn from_file(path: &Path, vocab_size: usize) -> Result<Self, Rknn3Error> {
        let data = std::fs::read(path).map_err(|e| {
            Rknn3Error::EmbeddingFailed(format!("failed to read {}: {}", path.display(), e))
        })?;
        if data.is_empty() {
            return Err(Rknn3Error::EmbeddingFailed(format!(
                "embedding file is empty: {}",
                path.display()
            )));
        }
        let embedding_dim = (data.len() / vocab_size) / 2;
        tracing::info!(
            "Embedding loaded: {} bytes, dim={}",
            data.len(),
            embedding_dim
        );
        Ok(Self {
            data,
            vocab_size,
            embedding_dim,
        })
    }
}

// ---------------------------------------------------------------------------
// InferenceCallbacks — LlmCallbacks impl with channel output
// ---------------------------------------------------------------------------

struct InferenceCallbacks {
    id: String,
    tokenizer: shimmytok::Tokenizer,
    embedding: Embedding,
    sender: Arc<StdMutex<tokio::sync::mpsc::UnboundedSender<StreamChunk>>>,
    curr_tokens: usize,
    max_tokens: usize,
    decoder: ByteBufferedDecoder,
}

impl aarch64_rknn::prelude::LlmCallbacks for InferenceCallbacks {
    fn tokenize(&self, text: &str, buf: &mut [i32]) -> Result<usize, aarch64_rknn::prelude::CallbackError> {
        match self.tokenizer.encode(text, false) {
            Ok(ids) => {
                let len = ids.len().min(buf.len());
                for (i, &id) in ids.iter().take(len).enumerate() {
                    buf[i] = id as i32;
                }
                Ok(len)
            }
            Err(_) => Err(aarch64_rknn::prelude::CallbackError::TokenizeFailed),
        }
    }

    fn embed(&self, tokens: &[i32], buf: &mut [u8]) -> Result<(), aarch64_rknn::prelude::CallbackError> {
        let expected_len = tokens.len() * self.embedding.embedding_dim * 2;
        if buf.len() != expected_len {
            return Err(aarch64_rknn::prelude::CallbackError::InvalidInput);
        }
        for (n, &token_id) in tokens.iter().enumerate() {
            if token_id < 0 || (token_id as usize) >= self.embedding.vocab_size {
                return Err(aarch64_rknn::prelude::CallbackError::InvalidInput);
            }
            let src_offset = token_id as usize * self.embedding.embedding_dim * 2;
            let dst_offset = n * self.embedding.embedding_dim * 2;
            let copy_len = self.embedding.embedding_dim * 2;
            buf[dst_offset..dst_offset + copy_len]
                .copy_from_slice(&self.embedding.data[src_offset..src_offset + copy_len]);
        }
        Ok(())
    }

    fn on_result(&mut self, token_ids: &[i32], state: aarch64_rknn::prelude::LlmCallState) {
        use aarch64_rknn::prelude::LlmCallState;
        match state {
            LlmCallState::Normal => {
                trace!(
                    "[{}] [rknn3] on_result state=Normal, token_ids={:?}",
                    self.id, token_ids
                );
                let pieces: Vec<String> = token_ids
                    .iter()
                    .filter(|&&tid| !self.tokenizer.is_special_token(tid as u32))
                    .filter_map(|&tid| self.tokenizer.token_to_piece(tid as u32).ok())
                    .collect();
                if pieces.is_empty() {
                    return;
                }
                let refs: Vec<&str> = pieces.iter().map(String::as_str).collect();
                let text = self.decoder.feed(&refs);
                if self.curr_tokens >= self.max_tokens {
                    let _ = self.sender.lock().unwrap().send(StreamChunk::Finish(FinishReason::Length));
                    return;
                }
                let send_result = self.sender.lock().unwrap().send(StreamChunk::Normal(LLMSteamChunkType::RawToken(text)));
                if send_result.is_err() {
                    return;
                }
                self.curr_tokens += pieces.len();
            }
            LlmCallState::Finish => {
                debug!("[{}] [rknn3] on_result state=Finish", self.id);
                let remaining = self.decoder.flush();
                if !remaining.is_empty() {
                    let _ = self.sender.lock().unwrap().send(StreamChunk::Normal(LLMSteamChunkType::RawToken(remaining)));
                }
                let _ = self.sender.lock().unwrap().send(StreamChunk::Finish(FinishReason::Stop));
            }
            LlmCallState::Error => {
                error!("[{}] [rknn3] on_result state=Error", self.id);
                let _ = self.sender.lock().unwrap().send(StreamChunk::Error("inference error".to_string()));
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// LLM 实例（Context + Session）
// ---------------------------------------------------------------------------

struct LlmInstance {
    _ctx: aarch64_rknn::prelude::Context,
    session: aarch64_rknn::prelude::Session,
}

/// LLM 池
type LlmPool = UnmanagedPool<LlmInstance>;

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn load_tokenizer_and_embedding(
    model_path: &Path,
    vocab_size: usize,
) -> Result<(shimmytok::Tokenizer, Embedding), Rknn3Error> {
    let gguf_path = model_path.join("model.tokenizer.gguf");
    let tokenizer = shimmytok::Tokenizer::from_gguf_file(&gguf_path).map_err(|e| {
        Rknn3Error::TokenizerFailed(format!(
            "failed to load tokenizer {}: {}",
            gguf_path.display(),
            e
        ))
    })?;
    tracing::info!("Tokenizer loaded, vocab_size={}", tokenizer.vocab_size());

    let embed_path = model_path.join("model.embed.bin");
    let embedding = Embedding::from_file(&embed_path, vocab_size)?;

    Ok((tokenizer, embedding))
}

fn build_llm_params(options: &Rknn3Options) -> Result<aarch64_rknn::prelude::LlmParams, Rknn3Error> {
    let bos_ids: Vec<i32> = options.special_bos_id.clone();
    let eos_ids: Vec<i32> = options.special_eos_id.clone();

    let params = aarch64_rknn::prelude::LlmParams::new(&options.logits_name, options.vocab_size)
        .map_err(|e| Rknn3Error::ModelInitFailed(format!("LlmParams::new failed: {}", e)))?
        .max_context_len(options.context_size as i32)
        .top_k(options.top_k as i32)
        .top_p(options.top_p)
        .temperature(options.temperature)
        .repeat_penalty(options.repeat_penalty)
        .special_bos_id(&bos_ids)
        .special_eos_id(&eos_ids)
        .linefeed_id(options.linefeed_id)
        .skip_special_token(options.skip_special_token);
    Ok(params)
}

fn setup_session(
    ctx: &aarch64_rknn::prelude::Context,
    options: &Rknn3Options,
    model_path: &Path,
) -> Result<aarch64_rknn::prelude::Session, Rknn3Error> {
    let llm_param = build_llm_params(options)?;
    let mut session = aarch64_rknn::prelude::Session::new(ctx, &mut [llm_param])
        .map_err(|e| Rknn3Error::ModelInitFailed(format!("rknn3_session_init failed: {}", e)))?;

    let (tokenizer, embedding) =
        load_tokenizer_and_embedding(model_path, options.vocab_size as usize)?;

    let callbacks = InferenceCallbacks {
        id: String::new(),
        tokenizer,
        embedding,
        sender: Arc::new(StdMutex::new(tokio::sync::mpsc::unbounded_channel().0)),
        curr_tokens: 0,
        max_tokens: options.context_size,
        decoder: ByteBufferedDecoder::new(),
    };

    session.set_callback(Box::new(callbacks))
        .map_err(|e| Rknn3Error::ModelInitFailed(format!("set_callback failed: {}", e)))?;

    Ok(session)
}

// ---------------------------------------------------------------------------
// Rknn3Backend - RKNN3 NPU 推理后端
// ---------------------------------------------------------------------------

pub struct Rknn3Backend {
    llm_pool: LlmPool,
    model_config: ModelConfig,
    options: Rknn3Options,
}

impl Rknn3Backend {
    /// 创建 LLM 池
    fn create_llm_pool(model_path: &Path, options: &Rknn3Options) -> Result<LlmPool, Rknn3Error> {
        let pool_size = options.max_concurrency.max(1);

        let core_mask = u32::from_str_radix(options.core_mask.trim_start_matches("0x"), 16).unwrap_or(0xff);

        // 1. 初始化主 Context
        let ctx = aarch64_rknn::prelude::Context::new()
            .map_err(|e| Rknn3Error::ModelInitFailed(format!("rknn3_init failed: {}", e)))?;

        // 2. 加载模型
        let model_file = model_path.join("model.rknn");
        let weight_file = model_path.join("model.weight");

        ctx.load_model(
            model_file.to_str().ok_or_else(|| Rknn3Error::InvalidModelPath(format!("{:?}", model_file)))?,
            weight_file.to_str().ok_or_else(|| Rknn3Error::InvalidModelPath(format!("{:?}", weight_file)))?,
        )
        .map_err(|e| Rknn3Error::ModelInitFailed(format!("load_model failed: {}", e)))?;

        // 3. Model init
        let mut config = aarch64_rknn::prelude::ModelConfig::new().core_mask(core_mask);
        ctx.model_init(&mut config)
            .map_err(|e| Rknn3Error::ModelInitFailed(format!("model_init failed: {}", e)))?;

        // 4. 验证 context_size
        let llm_cfg = ctx.query_llm_config()
            .map_err(|e| Rknn3Error::ModelInitFailed(format!("query_llm_config failed: {}", e)))?;

        let context_size = options.context_size as u32;
        if context_size > llm_cfg.max_ctx_len {
            return Err(Rknn3Error::ModelInitFailed(format!(
                "context_size ({}) exceeds max_ctx_len ({})",
                context_size, llm_cfg.max_ctx_len
            )));
        }

        tracing::info!(
            "LLM config: max_ctx_len={}, max_position_embeddings={}, context_size={}",
            llm_cfg.max_ctx_len, llm_cfg.max_position_embeddings, context_size
        );

        // 5. 创建主 Session
        let primary_session = setup_session(&ctx, options, model_path)?;

        // 6. dup_context 复制（串行，FFI 非线程安全）
        let mut dup_contexts: Vec<aarch64_rknn::prelude::Context> = Vec::with_capacity(pool_size.saturating_sub(1));
        for i in 1..pool_size {
            let dup_ctx = ctx.dup_context()
                .unwrap_or_else(|e| panic!("dup_context failed for instance {}: {}", i, e));
            dup_contexts.push(dup_ctx);
        }

        let mut instances = vec![LlmInstance { _ctx: ctx, session: primary_session }];

        for (i, dup_ctx) in dup_contexts.into_iter().enumerate() {
            let dup_session = setup_session(&dup_ctx, options, model_path)
                .unwrap_or_else(|e| panic!("setup_session on dup_context {} failed: {}", i + 1, e));
            instances.push(LlmInstance { _ctx: dup_ctx, session: dup_session });
        }

        // 7. 创建池
        let config = PoolConfig {
            max_size: pool_size,
            timeout: Some(Duration::from_secs(options.pool_timeout_secs)),
            runtime: Some(DeadpoolRuntime::Tokio1),
        };

        let pool = UnmanagedPool::from_config(&config);
        for instance in instances {
            pool.try_add(instance)
                .unwrap_or_else(|(_inst, e)| panic!("Failed to add instance to pool: {}", e));
        }

        Ok(pool)
    }
}

#[async_trait]
impl BaseBackend for Rknn3Backend {
    async fn init(model_config: &ModelConfig) -> Result<Self, BackendError> {
        let options: Rknn3Options = Rknn3Options::default();

        let llm_pool = Self::create_llm_pool(&model_config.model_path, &options)
            .map_err(Rknn3Error::from)?;

        Ok(Self {
            llm_pool,
            model_config: model_config.clone(),
            options,
        })
    }

    fn name(&self) -> &'static str {
        "rknn3"
    }

    fn health_check(&self) -> bool {
        true
    }

    fn resource_usage(&self) -> Vec<ResourceType> {
        vec![]
    }
}

#[async_trait]
impl LLMBackend for Rknn3Backend {
    async fn inference_stream(
        &self,
        input: LLMInferenceInput,
        options: LLMInferenceOptions,
    ) -> Result<DroppableReceiver<StreamChunk>, BackendError> {
        let id = options.id.clone();

        // 只支持 Tokens 输入
        let token_ids: Vec<i32> = match input {
            LLMInferenceInput::Tokens(tokens) => tokens.iter().map(|&t| t as i32).collect(),
            LLMInferenceInput::Messages(_messages) => {
                return Err(BackendError::UnsupportedInput(
                    "RKNN3 backend only supports token input".to_string(),
                ));
            }
        };

        tracing::info!("[{}] [rknn3] Acquiring model", id);

        // 获取池实例
        let mut instance = self.llm_pool.get().await.map_err(|e| {
            Rknn3Error::LlmBusy(format!("Failed to acquire LLM handle: {}", e))
        })?;

        tracing::info!("[{}] [rknn3] Acquired model", id);

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();

        // 加载 tokenizer 和 embedding
        let (tokenizer, embedding) = load_tokenizer_and_embedding(
            &self.model_config.model_path,
            self.options.vocab_size as usize,
        ).map_err(Rknn3Error::from)?;

        let max_tokens = self.options.context_size;
        let sender_arc = Arc::new(StdMutex::new(sender));
        let error_sender = sender_arc.clone();

        let cb = InferenceCallbacks {
            id: id.clone(),
            tokenizer,
            embedding,
            sender: sender_arc,
            curr_tokens: 0,
            max_tokens,
            decoder: ByteBufferedDecoder::new(),
        };

        instance.session.set_callback(Box::new(cb)).map_err(|e| {
            Rknn3Error::InferenceError(format!("set_callback failed: {}", e))
        })?;

        // 创建 stop handle
        let stop_handle = instance.session.stop_handle();

        let token_count = token_ids.len();
        let mut llm_input = aarch64_rknn::prelude::LlmInput::tokens(token_ids)
            .role("user")
            .unwrap();

        if options.enable_thinking.unwrap_or(false) {
            llm_input = llm_input.enable_thinking(true);
        }

        let mut infer_param = aarch64_rknn::prelude::InferParams::new()
            .max_new_tokens(max_tokens as i32)
            .keep_history(false);

        let id_for_thread = id.clone();
        let id_for_abort = id.clone();

        tokio::task::spawn_blocking(move || {
            tracing::info!(
                "[{}] [rknn3] Start inference, input tokens: {}",
                id_for_thread, token_count
            );

            let result = instance.session.run(&mut [llm_input], &mut infer_param);

            if let Err(e) = result {
                let _ = error_sender.lock().unwrap().send(StreamChunk::Error(format!("rknn3 run error: {}", e)));
            }

            tracing::info!("[{}] [rknn3] Inference finished", id_for_thread);
        });

        Ok(DroppableReceiver::new(
            receiver,
            Some(Box::new(move || {
                tracing::trace!("[{}] [rknn3] Abort requested", id_for_abort);
                stop_handle.stop().ok();
                Ok(())
            })),
        ))
    }

    async fn context_size(&self) -> Result<usize, BackendError> {
        Ok(self.options.context_size)
    }
}