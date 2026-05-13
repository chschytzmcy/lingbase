//! Backend factory for creating and selecting inference backends.

use std::path::Path;
use std::sync::Arc;
use crate::error::{InferenceError, InferenceResult};
use super::InferenceBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Cuda,
    Cpu,
    Rkllm,
}

impl Default for BackendType {
    fn default() -> Self {
        BackendType::Cpu
    }
}

pub struct BackendFactory;

impl BackendFactory {
    pub fn create(backend_type: BackendType, model_path: &Path, n_ctx: i32) -> InferenceResult<Arc<dyn InferenceBackend>> {
        match backend_type {
            BackendType::Cuda => {
                #[cfg(feature = "cuda")]
                {
                    let backend = super::cuda::CudaBackend::new(model_path, n_ctx)?;
                    return Ok(Arc::new(backend));
                }
                #[cfg(not(feature = "cuda"))]
                Err(InferenceError::BackendNotAvailable("CUDA".to_string()))
            }
            BackendType::Cpu => {
                let backend = super::cpu::CpuBackend::new(model_path, n_ctx)?;
                Ok(Arc::new(backend))
            }
            BackendType::Rkllm => {
                #[cfg(feature = "rkllm")]
                {
                    let backend = super::rkllm::RkllmBackend::new(model_path, n_ctx)?;
                    return Ok(Arc::new(backend));
                }
                #[cfg(not(feature = "rkllm"))]
                Err(InferenceError::BackendNotAvailable("RKLLM".to_string()))
            }
        }
    }

    pub fn auto_detect() -> BackendType {
        #[cfg(feature = "cuda")]
        if Self::cuda_available() {
            return BackendType::Cuda;
        }
        BackendType::Cpu
    }

    #[cfg(feature = "cuda")]
    fn cuda_available() -> bool {
        true
    }

    #[cfg(not(feature = "cuda"))]
    fn cuda_available() -> bool {
        false
    }
}