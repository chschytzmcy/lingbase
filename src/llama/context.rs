//! LlamaContext wrapper - manages inference context.

use crate::error::{InferenceResult, InferenceError};

#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct llama_context_params {
    pub n_ctx: i32,
    pub n_parts: i32,
    pub n_gpu_layers: i32,
    pub seed: u32,
    pub logits_all: bool,
    pub embedding: bool,
    pub n_threads: u32,
    pub n_threads_batch: u32,
    pub flash: bool,
    pub auto_continue: bool,
}

pub type llama_token = i32;

extern "C" {
    fn llama_new_context_with_model(model: *mut libc::c_void, params: llama_context_params) -> *mut libc::c_void;
    fn llama_free_ctx(ctx: *mut libc::c_void);
    fn llama_kv_cache_clear(ctx: *mut libc::c_void);
    fn llama_kv_cache_seq_rm(ctx: *mut libc::c_void, seq_from: i32, seq_to: i32, p0: i32);
}

pub struct LlamaContext {
    initialized: bool,
}

impl LlamaContext {
    pub fn new(_model_ptr: *mut libc::c_void, _n_ctx: i32, _n_threads: u32) -> InferenceResult<Self> {
        Err(InferenceError::BackendNotInitialized)
    }

    pub fn clear_cache(&self) {}

    pub fn seq_remove(&self, _seq_from: i32, _seq_to: i32, _p0: i32) {}
}

impl Drop for LlamaContext {
    fn drop(&mut self) {}
}