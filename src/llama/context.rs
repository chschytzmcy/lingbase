//! LlamaContext wrapper - manages inference context.

use crate::error::{InferenceResult, InferenceError};
use super::ffi::{
    ContextPtr, ModelPtr,
    llama_context_default_params, llama_init_from_model, llama_free,
    llama_get_logits, llama_n_ctx,
    LlamaBatch,
};

pub struct LlamaContext {
    ptr: Option<ContextPtr>,
    #[allow(dead_code)]
    n_ctx: u32,
}

unsafe impl Send for LlamaContext {}
unsafe impl Sync for LlamaContext {}

impl LlamaContext {
    pub fn new(model_ptr: ModelPtr, n_ctx: u32, n_threads: u32) -> InferenceResult<Self> {
        if model_ptr.is_null() {
            return Err(InferenceError::ModelNotLoaded);
        }

        let mut params = unsafe { llama_context_default_params() };
        params.n_ctx = n_ctx;
        params.n_threads = n_threads as i32;
        params.n_threads_batch = n_threads as i32;

        let ctx_ptr = unsafe { llama_init_from_model(model_ptr, params) };

        if ctx_ptr.is_null() {
            return Err(InferenceError::BackendError(
                "Failed to create llama context".to_string()
            ));
        }

        let actual_n_ctx = unsafe { llama_n_ctx(ctx_ptr) };

        Ok(Self {
            ptr: Some(ctx_ptr),
            n_ctx: actual_n_ctx,
        })
    }

    pub fn ptr(&self) -> Option<ContextPtr> {
        self.ptr
    }

    pub fn decode(&self, batch: LlamaBatch) -> InferenceResult<()> {
        let ctx = self.ptr.ok_or(InferenceError::BackendNotInitialized)?;
        let result = unsafe { super::ffi::llama_decode(ctx, batch) };
        if result != 0 {
            return Err(InferenceError::InferenceFailed(
                format!("llama_decode failed with code {}", result)
            ));
        }
        Ok(())
    }

    pub fn get_logits(&self, n_vocab: usize) -> Option<&[f32]> {
        let ctx = self.ptr?;
        let logits = unsafe { llama_get_logits(ctx) };
        if logits.is_null() {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts(logits, n_vocab) })
        }
    }

    pub fn clear_cache(&self) {
        // Memory management handled by context
    }
}

impl Drop for LlamaContext {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            unsafe { llama_free(ptr) };
        }
    }
}