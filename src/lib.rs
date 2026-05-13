//! Lingbase - Edge LLM Inference Service Library

pub mod api;
pub mod backend;
pub mod llama;
pub mod infra;
pub mod error;

pub use error::{InferenceError, InferenceResult};
pub use backend::{InferenceBackend, ForwardResult, BackendFactory, BackendType, InferenceConfig};
pub use infra::{AppConfig, HealthCheck, init_logging};