//! 后端 Trait 定义
//!
//! 定义三层后端抽象：
//! - BaseBackend: 所有后端的公共接口（初始化、资源）
//! - LLMBackend: LLM 推理后端接口（继承 BaseBackend）

use async_trait::async_trait;
use futures::Stream;

use crate::api::types::Message;
use crate::infra::config::ModelConfig;
use crate::error::InferenceError;
use super::types::{FinishReason, ResourceType};
use super::error::BackendError;
use super::backend::ForwardResult;

/// 推理输入
#[derive(Debug, Clone)]
pub enum LLMInferenceInput {
    /// 预分词的 token IDs
    Tokens(Vec<u32>),
    /// 文本消息（需要内部 tokenize）
    Messages(Vec<Message>),
}

/// 推理选项
#[derive(Debug, Clone)]
pub struct LLMInferenceOptions {
    pub id: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: usize,
    pub repeat_penalty: f32,
    pub stream: bool,
    pub stop: Vec<String>,
    pub enable_thinking: Option<bool>,
}

impl Default for LLMInferenceOptions {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            max_tokens: 256,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            repeat_penalty: 1.1,
            stream: true,
            stop: vec![],
            enable_thinking: None,
        }
    }
}

/// 流式 Chunk 类型
#[derive(Debug, Clone)]
pub enum LLMSteamChunkType {
    /// 原始 token 文本
    RawToken(String),
    /// 带有元数据的 chunk
    Chunk { text: String, token_id: u32 },
}

/// 流式输出 Chunk
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// 正常 token
    Normal(LLMSteamChunkType),
    /// 推理完成
    Finish(FinishReason),
    /// 错误
    Error(String),
}

/// 可丢弃的接收者，用于流式输出
///
/// Drop 时自动调用 abort_fn 终止推理
pub struct DroppableReceiver<T: Send> {
    receiver: Option<tokio::sync::mpsc::UnboundedReceiver<T>>,
    abort_fn: Option<Box<dyn Fn() -> Result<(), BackendError> + Send + Sync>>,
}

impl<T: Send> DroppableReceiver<T> {
    pub fn new(
        receiver: tokio::sync::mpsc::UnboundedReceiver<T>,
        abort_fn: Option<Box<dyn Fn() -> Result<(), BackendError> + Send + Sync>>,
    ) -> Self {
        Self {
            receiver: Some(receiver),
            abort_fn,
        }
    }

    pub fn into_stream(&mut self) -> impl Stream<Item = T> + Send {
        tokio_stream::wrappers::UnboundedReceiverStream::new(self.receiver.take().unwrap())
    }
}

impl<T: Send> Drop for DroppableReceiver<T> {
    fn drop(&mut self) {
        if let Some(abort_fn) = self.abort_fn.take() {
            let _ = abort_fn();
        }
    }
}

/// BaseBackend - 所有后端的公共 trait
///
/// 定义初始化和资源查询接口，与具体硬件解耦
#[async_trait]
pub trait BaseBackend: Send + Sync {
    /// 异步初始化
    async fn init(model_config: &ModelConfig) -> Result<Self, BackendError>
    where
        Self: Sized;

    /// 获取后端名称
    fn name(&self) -> &'static str;

    /// 健康检查
    fn health_check(&self) -> bool;

    /// 资源使用情况
    fn resource_usage(&self) -> Vec<ResourceType>;
}

/// LLMBackend - LLM 推理后端 trait
///
/// 继承 BaseBackend，提供 LLM 推理能力
#[async_trait]
pub trait LLMBackend: BaseBackend {
    /// 流式推理
    async fn inference_stream(
        &self,
        input: LLMInferenceInput,
        options: LLMInferenceOptions,
    ) -> Result<DroppableReceiver<StreamChunk>, BackendError>;

    /// 非流式推理
    async fn inference(
        &self,
        input: LLMInferenceInput,
        options: LLMInferenceOptions,
    ) -> Result<ForwardResult, InferenceError> {
        let mut receiver = self.inference_stream(input, options).await?;

        use futures::StreamExt;
        let mut stream = receiver.into_stream();
        let tokens = Vec::new();
        let mut first_token_ms = None;
        let start = std::time::Instant::now();

        while let Some(chunk) = stream.next().await {
            match chunk {
                StreamChunk::Normal(LLMSteamChunkType::RawToken(_)) => {
                    if first_token_ms.is_none() {
                        first_token_ms = Some(start.elapsed().as_millis() as u64);
                    }
                }
                StreamChunk::Normal(LLMSteamChunkType::Chunk { .. }) => {}
                StreamChunk::Finish(_) => {}
                StreamChunk::Error(e) => return Err(InferenceError::InferenceFailed(e)),
            }
        }

        Ok(ForwardResult {
            tokens,
            first_token_ms,
            total_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// 上下文窗口大小
    async fn context_size(&self) -> Result<usize, BackendError>;

    /// 是否支持 logits（默认 false）
    fn supports_logits(&self) -> bool {
        false
    }
}