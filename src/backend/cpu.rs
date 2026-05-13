//! CPU backend implementation using llama.cpp.

use std::path::Path;
use std::time::Instant;
use crate::error::InferenceResult;
use crate::llama::{LlamaModel, LlamaContext, Tokenizer};
use super::{InferenceBackend, MemoryStats, InferenceConfig, ForwardResult};

pub struct CpuBackend {
    model: Option<LlamaModel>,
    context: Option<LlamaContext>,
    tokenizer: Option<Tokenizer>,
    n_ctx: i32,
}

impl CpuBackend {
    pub fn new<P: AsRef<Path>>(model_path: P, n_ctx: i32) -> InferenceResult<Self> {
        let model = LlamaModel::from_file(model_path, 0)?;
        let context = LlamaContext::new(std::ptr::null_mut(), n_ctx, 4)?;
        let tokenizer = Tokenizer::new(std::ptr::null_mut());

        Ok(Self {
            model: Some(model),
            context: Some(context),
            tokenizer: Some(tokenizer),
            n_ctx,
        })
    }

    pub fn is_initialized(&self) -> bool {
        self.model.as_ref().map(|m| m.is_loaded()).unwrap_or(false)
            && self.context.is_some()
    }
}

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

    fn forward(&self, tokens: &[i32], _config: &InferenceConfig) -> InferenceResult<ForwardResult> {
        let start = Instant::now();
        let all_tokens = tokens.to_vec();
        let first_token_ms = None;

        Ok(ForwardResult {
            tokens: all_tokens,
            first_token_ms,
            total_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn max_context_size(&self) -> usize {
        self.n_ctx as usize
    }

    fn sample_token(&self, _logits: &[f32], _config: &InferenceConfig) -> i32 {
        0
    }
}