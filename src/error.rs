//! Unified error types for the inference service.

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum InferenceError {
    #[error("Failed to load model from {0}: file not found or invalid format")]
    ModelLoadFailed(String),

    #[error("Invalid model path: {0}")]
    InvalidPath(String),

    #[error("Model format not supported: {0}")]
    UnsupportedModelFormat(String),

    #[error("Tokenization failed: {0}")]
    TokenizationFailed(String),

    #[error("Prompt too long: {0} tokens (max: {1})")]
    PromptTooLong(usize, usize),

    #[error("Inference timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    #[error("No tokens generated")]
    NoTokensGenerated,

    #[error("Backend '{0}' not available")]
    BackendNotAvailable(String),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("Backend not initialized")]
    BackendNotInitialized,

    #[error("Out of memory: {0}")]
    OutOfMemory(String),

    #[error("Memory allocation failed")]
    MemoryAllocationFailed,

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Model not loaded")]
    ModelNotLoaded,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("FFI error: {0}")]
    FfiError(String),
}

impl InferenceError {
    pub fn error_code(&self) -> &'static str {
        match self {
            InferenceError::ModelLoadFailed(_) => "MODEL_LOAD_FAILED",
            InferenceError::InvalidPath(_) => "INVALID_PATH",
            InferenceError::UnsupportedModelFormat(_) => "UNSUPPORTED_FORMAT",
            InferenceError::TokenizationFailed(_) => "TOKENIZATION_FAILED",
            InferenceError::PromptTooLong(_, _) => "PROMPT_TOO_LONG",
            InferenceError::Timeout(_) => "TIMEOUT",
            InferenceError::InferenceFailed(_) => "INFERENCE_FAILED",
            InferenceError::NoTokensGenerated => "NO_TOKENS",
            InferenceError::BackendNotAvailable(_) => "BACKEND_NOT_AVAILABLE",
            InferenceError::BackendError(_) => "BACKEND_ERROR",
            InferenceError::BackendNotInitialized => "BACKEND_NOT_INITIALIZED",
            InferenceError::OutOfMemory(_) => "OUT_OF_MEMORY",
            InferenceError::MemoryAllocationFailed => "MEMORY_ALLOCATION_FAILED",
            InferenceError::HealthCheckFailed(_) => "HEALTH_CHECK_FAILED",
            InferenceError::ModelNotLoaded => "MODEL_NOT_LOADED",
            InferenceError::Internal(_) => "INTERNAL_ERROR",
            InferenceError::FfiError(_) => "FFI_ERROR",
        }
    }

    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            InferenceError::ModelLoadFailed(_)
                | InferenceError::InferenceFailed(_)
                | InferenceError::BackendError(_)
                | InferenceError::OutOfMemory(_)
                | InferenceError::MemoryAllocationFailed
                | InferenceError::Internal(_)
                | InferenceError::FfiError(_)
        )
    }
}

pub type InferenceResult<T> = Result<T, InferenceError>;