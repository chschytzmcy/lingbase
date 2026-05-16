//! LlamaModel wrapper - manages model loading and lifecycle.

use std::path::Path;
use crate::error::{InferenceError, InferenceResult};
use super::ffi::{
    ModelPtr, VocabPtr,
    llama_model_default_params, llama_model_load_from_file,
    llama_model_free, llama_model_get_vocab,
    llama_model_n_ctx_train, llama_vocab_n_tokens, ggml_backend_load_all_from_path,
};

pub struct LlamaModel {
    ptr: Option<ModelPtr>,
    vocab: Option<VocabPtr>,
    n_vocab: i32,
    n_ctx_train: i32,
}

unsafe impl Send for LlamaModel {}
unsafe impl Sync for LlamaModel {}

impl LlamaModel {
    /// Load model from file with specified backend library directory.
    ///
    /// # Arguments
    /// * `path` - Path to the GGUF model file
    /// * `n_gpu_layers` - Number of layers to offload to GPU (0 for CPU only)
    /// * `lib_dir` - Directory containing llama.cpp libraries (e.g., "lib/x86_64" or "lib/cuda")
    pub fn from_file_with_backend<P: AsRef<Path>>(
        path: P,
        n_gpu_layers: i32,
        lib_dir: &std::path::Path,
    ) -> InferenceResult<Self> {
        use std::ffi::CString;

        // Load ggml backends from the specified library directory
        let lib_dir_c = CString::new(lib_dir.to_string_lossy().as_bytes()).unwrap();
        unsafe { ggml_backend_load_all_from_path(lib_dir_c.as_ptr()) };

        let path_str = path.as_ref().to_string_lossy().into_owned();

        let path_c = CString::new(path_str)
            .map_err(|e| InferenceError::InvalidPath(e.to_string()))?;

        let mut params = unsafe { llama_model_default_params() };
        params.n_gpu_layers = n_gpu_layers;
        params.use_mmap = true;
        params.use_mlock = false;

        let model_ptr = unsafe {
            llama_model_load_from_file(path_c.as_ptr(), params)
        };

        if model_ptr.is_null() {
            return Err(InferenceError::ModelLoadFailed(
                path.as_ref().display().to_string()
            ));
        }

        let vocab_ptr = unsafe { llama_model_get_vocab(model_ptr) };
        let n_vocab = if vocab_ptr.is_null() {
            0
        } else {
            unsafe { llama_vocab_n_tokens(vocab_ptr) }
        };
        let n_ctx_train = unsafe { llama_model_n_ctx_train(model_ptr) };

        Ok(Self {
            ptr: Some(model_ptr),
            vocab: Some(vocab_ptr),
            n_vocab,
            n_ctx_train,
        })
    }

    /// Load model from file using CPU backend (lib/x86_64).
    /// This is a convenience method that calls from_file_with_backend with default CPU lib path.
    pub fn from_file<P: AsRef<Path>>(path: P, n_gpu_layers: i32) -> InferenceResult<Self> {
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let lib_dir = match arch.as_str() {
            "x86_64" => "lib/x86_64",
            "aarch64" => "lib/aarch64",
            _ => "lib/x86_64",
        };

        // Get the directory containing the executable
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();

        // Try both: relative to exe and relative to current dir
        let lib_path = exe_dir.join(&lib_dir);
        let fallback_path = std::path::Path::new(&lib_dir);

        if lib_path.exists() {
            Self::from_file_with_backend(path, n_gpu_layers, &lib_path)
        } else {
            Self::from_file_with_backend(path, n_gpu_layers, fallback_path)
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.ptr.is_some()
    }

    pub fn ptr(&self) -> Option<ModelPtr> {
        self.ptr
    }

    pub fn vocab_ptr(&self) -> Option<VocabPtr> {
        self.vocab
    }

    pub fn n_vocab(&self) -> i32 {
        self.n_vocab
    }

    pub fn n_ctx_train(&self) -> i32 {
        self.n_ctx_train
    }
}

impl Drop for LlamaModel {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            unsafe { llama_model_free(ptr) };
        }
    }
}