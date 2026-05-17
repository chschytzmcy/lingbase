//! Backend factory for creating and selecting inference backends.

use std::path::Path;
use std::sync::Arc;
use crate::error::{InferenceError, InferenceResult};
use super::InferenceBackend;
use super::types::BackendType;

pub struct BackendFactory;

impl BackendFactory {
    pub fn create(backend_type: BackendType, model_path: &Path, n_ctx: i32) -> InferenceResult<Arc<dyn InferenceBackend>> {
        match backend_type {
            BackendType::LlamaCuda => {
                #[cfg(feature = "cuda")]
                {
                    let backend = super::cuda::CudaBackend::new(model_path, n_ctx)?;
                    return Ok(Arc::new(backend));
                }
                #[cfg(not(feature = "cuda"))]
                Err(InferenceError::BackendNotAvailable("CUDA".to_string()))
            }
            BackendType::LlamaCpu => {
                let backend = super::cpu::CpuBackend::new(model_path, n_ctx)?;
                Ok(Arc::new(backend))
            }
            BackendType::Rknn3 => Err(InferenceError::BackendNotAvailable(
                "RKNN3 backend is managed by BackendManager, not BackendFactory".to_string()
            )),
            BackendType::Proxy => Err(InferenceError::BackendNotAvailable("Proxy backend not yet implemented".to_string()))
        }
    }

    pub fn auto_detect() -> BackendType {
        #[cfg(feature = "cuda")]
        if Self::cuda_available() {
            return BackendType::LlamaCuda;
        }
        BackendType::LlamaCpu
    }

    #[cfg(feature = "cuda")]
    fn cuda_available() -> bool {
        true
    }

    #[cfg(not(feature = "cuda"))]
    #[allow(dead_code)]
    fn cuda_available() -> bool {
        false
    }
}