//! Batch management for llama.cpp

use crate::error::InferenceResult;
use crate::llama::ffi::{LlamaBatch, LlamaToken, llama_batch_get_one};

/// Wrapper that keeps batch and its underlying token memory alive together
pub struct BatchWithTokens {
    pub batch: LlamaBatch,
    #[allow(dead_code)]
    tokens: Vec<LlamaToken>,
}

impl BatchWithTokens {
    /// Create a batch for a single sequence from token array
    pub fn new(tokens: &[LlamaToken]) -> InferenceResult<Self> {
        let mut tokens_copy = tokens.to_vec();
        let batch = unsafe { llama_batch_get_one(tokens_copy.as_mut_ptr(), tokens.len() as i32) };
        Ok(Self {
            batch,
            tokens: tokens_copy,
        })
    }

    pub fn into_batch(self) -> LlamaBatch {
        self.batch
    }
}

impl LlamaBatch {
    pub fn n_tokens(&self) -> i32 {
        self.n_tokens
    }
}