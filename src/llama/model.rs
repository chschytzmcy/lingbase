//! LlamaModel wrapper - manages model loading and lifecycle.
//!
//! Note: This module contains FFI bindings to llama.cpp C API.
//! Without a compiled llama.cpp library, these will panic at runtime.

use std::path::Path;
use crate::error::{InferenceError, InferenceResult};

#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct llama_model_params {
    pub n_gpu_layers: i32,
    pub main_gpu: i32,
    pub tensor_split: *const f32,
    pub rpc_servers: *const libc::c_char,
    pub progress_callback: Option<unsafe extern "C" fn(progress: f32, ctx: *mut libc::c_void)>,
    pub progress_callback_user_data: *mut libc::c_void,
    pub kv_overrides: *mut libc::c_void,
    pub spa: bool,
    pub mul_mat: bool,
    pub f16_kv: bool,
    pub use_mmap: bool,
    pub use_mlock: bool,
    pub va: bool,
}

extern "C" {
    fn llama_model_default_params() -> llama_model_params;
    fn llama_load_model_file(path: *const libc::c_char, params: llama_model_params) -> *mut libc::c_void;
    fn llama_free_model(ctx: *mut libc::c_void);
    fn llama_model_is_loaded(ctx: *mut libc::c_void) -> bool;
    fn llama_model_n_ctx_train(ctx: *mut libc::c_void) -> i32;
}

/// LlamaModel wraps a llama.cpp model instance
pub struct LlamaModel {
    loaded: bool,
}

impl LlamaModel {
    pub fn from_file<P: AsRef<Path>>(_path: P, _n_gpu_layers: i32) -> InferenceResult<Self> {
        // TODO: llama.cpp not compiled - return stub
        Err(InferenceError::ModelNotLoaded)
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn n_ctx_train(&self) -> i32 {
        4096
    }
}

impl Drop for LlamaModel {
    fn drop(&mut self) {
        // No-op since model not actually loaded
    }
}