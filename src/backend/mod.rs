//! Backend abstraction layer.

pub mod backend;
pub mod factory;
pub mod cpu;

#[cfg(feature = "cuda")]
pub mod cuda;

pub use backend::{InferenceBackend, MemoryStats, InferenceConfig, ForwardResult, StreamToken};
pub use factory::{BackendFactory, BackendType};
pub use cpu::CpuBackend;