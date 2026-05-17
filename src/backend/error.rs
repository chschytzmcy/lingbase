//! Backend error types

use thiserror::Error;
use crate::error::InferenceError;

/// Backend 错误类型
#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Backend not initialized")]
    BackendNotInitialized,

    #[error("Backend not available: {0}")]
    BackendNotAvailable(String),

    #[error("Unsupported input: {0}")]
    UnsupportedInput(String),

    #[error("Tokenizer failed: {0}")]
    TokenizerFailed(String),

    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    #[error("LLM is busy: {0}")]
    LLMBusy(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Unknown backend: {0}")]
    UnknownBackend(String),
}

impl From<BackendError> for InferenceError {
    fn from(e: BackendError) -> Self {
        InferenceError::BackendError(e.to_string())
    }
}