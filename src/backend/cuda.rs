//! CUDA backend (not yet implemented)

use std::path::Path;
use std::pin::Pin;
use futures::Stream;
use crate::error::{InferenceError, InferenceResult};
use super::{InferenceBackend, InferenceConfig, ForwardResult, StreamToken, MemoryStats};

pub struct CudaBackend;

impl CudaBackend {
    pub fn new(_model_path: &Path, _n_ctx: i32) -> InferenceResult<Self> {
        Err(InferenceError::BackendNotAvailable("CUDA backend not yet implemented".to_string()))
    }
}

impl InferenceBackend for CudaBackend {
    fn name(&self) -> &str {
        "cuda"
    }

    fn health_check(&self) -> bool {
        false
    }

    fn memory_stats(&self) -> MemoryStats {
        MemoryStats::default()
    }

    fn max_context_size(&self) -> usize {
        0
    }

    fn forward(&self, _tokens: &[i32], _config: &InferenceConfig) -> InferenceResult<ForwardResult> {
        Err(InferenceError::BackendNotAvailable("CUDA backend not yet implemented".to_string()))
    }

    fn forward_stream(&self, _tokens: &[i32], _config: &InferenceConfig) -> Pin<Box<dyn Stream<Item = InferenceResult<StreamToken>> + Send>> {
        Box::pin(futures::stream::empty())
    }

    fn tokenize(&self, _text: &str) -> InferenceResult<Vec<i32>> {
        Err(InferenceError::BackendNotAvailable("CUDA backend not yet implemented".to_string()))
    }

    fn detokenize(&self, _tokens: &[i32]) -> InferenceResult<String> {
        Err(InferenceError::BackendNotAvailable("CUDA backend not yet implemented".to_string()))
    }

    fn sample_token(&self, _logits: &[f32], _config: &InferenceConfig) -> i32 {
        0
    }
}
