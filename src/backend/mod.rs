//! Backend abstraction layer.

pub mod backend;
pub mod factory;
pub mod cpu;

pub use backend::{InferenceBackend, MemoryStats, InferenceConfig, ForwardResult};
pub use factory::{BackendFactory, BackendType};
pub use cpu::CpuBackend;