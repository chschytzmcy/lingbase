//! RKNN3 后端实现
//!
//! 基于 rknn3-sys 提供 LLM 推理能力。

use std::path::Path;

use async_trait::async_trait;
use deadpool::unmanaged::{Pool as UnmanagedPool, PoolConfig};
use tokio::sync::Mutex;
use tokio::time::Duration;

use crate::api::types::Message;
use crate::error::InferenceResult;
use crate::infra::config::ModelConfig;
use super::types::FinishReason;
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
}

impl From<Rknn3Error> for BackendError {
    fn from(e: Rknn3Error) -> Self {
        match e {
            Rknn3Error::ModelInitFailed(s) => BackendError::InferenceFailed(s),
            Rknn3Error::SessionError(s) => BackendError::InferenceFailed(s),
            Rknn3Error::LlmBusy(s) => BackendError::LLMBusy(s),
            Rknn3Error::InvalidModelPath(s) => BackendError::Internal(s),
        }
    }
}

/// LLM 实例（Context + Session）
struct LlmInstance {
    _ctx: rknn3_sys::prelude::Context,
    session: rknn3_sys::prelude::Session,
}

/// LLM 池
type LlmPool = UnmanagedPool<LlmInstance>;

/// Rknn3Backend - RKNN3 NPU 推理后端
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
        let ctx = rknn3_sys::prelude::Context::new()
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
        let mut config = rknn3_sys::prelude::ModelConfig::new().core_mask(core_mask);
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

        // 5. 创建 Session
        let session = create_session(&ctx, options)
            .map_err(|e| Rknn3Error::ModelInitFailed(format!("Session creation failed: {}", e)))?;

        // 6. dup_context 复制（串行，FFI 非线程安全）
        let mut instances = vec![LlmInstance { _ctx: ctx, session }];

        for i in 1..pool_size {
            let dup_ctx = instances[0]._ctx.dup_context()
                .unwrap_or_else(|e| panic!("dup_context failed for instance {}: {}", i, e));

            let dup_session = create_session(&dup_ctx, options)
                .unwrap_or_else(|e| panic!("Session creation on dup_context {} failed: {}", i, e));

            instances.push(LlmInstance { _ctx: dup_ctx, session: dup_session });
        }

        // 7. 创建池
        let config = PoolConfig {
            max_size: pool_size,
            timeout: Some(Duration::from_secs(options.pool_timeout_secs)),
            runtime: Some(deadpool::Runtime::Tokio1),
        };

        let pool = UnmanagedPool::from_config(&config);
        for instance in instances {
            pool.try_add(instance)
                .unwrap_or_else(|(_inst, e)| panic!("Failed to add instance to pool: {}", e));
        }

        Ok(pool)
    }
}

fn create_session(ctx: &rknn3_sys::prelude::Context, options: &Rknn3Options)
    -> Result<rknn3_sys::prelude::Session, Rknn3Error>
{
    let bos_ids: Vec<i32> = options.special_bos_id.clone();
    let eos_ids: Vec<i32> = options.special_eos_id.clone();

    let llm_params = rknn3_sys::prelude::LlmParams::new(&options.logits_name, options.vocab_size)
        .map_err(|e| Rknn3Error::ModelInitFailed(format!("LlmParams::new failed: {}", e)))?
        .max_context_len(options.context_size as i32)
        .top_k(options.top_k as i32)
        .top_p(options.top_p)
        .temperature(options.temperature)
        .repeat_penalty(options.repeat_penalty)
        .special_bos_id(&bos_ids)
        .special_eos_id(&eos_ids)
        .skip_special_token(options.skip_special_token);

    let mut session = rknn3_sys::prelude::Session::new(ctx, &mut [llm_params])
        .map_err(|e| Rknn3Error::SessionError(format!("Session::new failed: {}", e)))?;

    Ok(session)
}

#[async_trait]
impl BaseBackend for Rknn3Backend {
    async fn init(model_config: &ModelConfig) -> Result<Self, BackendError> {
        let options: Rknn3Options = Rknn3Options::default();

        let llm_pool = Self::create_llm_pool(&model_config.model_path, &options)
            .map_err(|e| e.into())?;

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

    fn resource_usage(&self) -> Vec<super::types::ResourceType> {
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
        // 只支持 Tokens 输入
        let token_ids: Vec<i32> = match input {
            LLMInferenceInput::Tokens(tokens) => tokens.iter().map(|&t| t as i32).collect(),
            LLMInferenceInput::Messages(_messages) => {
                return Err(BackendError::UnsupportedInput(
                    "RKNN3 backend only supports token input".to_string(),
                ));
            }
        };

        let max_tokens = self.options.context_size;

        // 获取池实例
        let mut instance = self.llm_pool.get().await.map_err(|e| {
            BackendError::from(Rknn3Error::LlmBusy(format!("Failed to acquire LLM handle: {}", e)))
        })?;

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();

        // 创建 stop handle
        let stop_handle = instance.session.stop_handle();

        let mut llm_input = rknn3_sys::prelude::LlmInput::tokens(token_ids)
            .role("user")
            .unwrap();

        if options.enable_thinking.unwrap_or(false) {
            llm_input = llm_input.enable_thinking(true);
        }

        let mut infer_param = rknn3_sys::prelude::InferParams::new()
            .max_new_tokens(max_tokens as i32)
            .keep_history(false);

        tokio::task::spawn_blocking(move || {
            let result = instance.session.run(&mut [llm_input], &mut infer_param);

            if let Err(e) = result {
                let _ = sender.send(StreamChunk::Error(format!("rknn3 run error: {}", e)));
            }

            // Session drop 后自动归还池
        });

        Ok(DroppableReceiver::new(
            receiver,
            Some(Box::new(move || {
                stop_handle.stop().ok();
                Ok(())
            })),
        ))
    }

    async fn context_size(&self) -> Result<usize, BackendError> {
        Ok(self.options.context_size)
    }
}