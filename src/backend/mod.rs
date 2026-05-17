//! Backend abstraction layer.
//!
//! 模块层次：
//!
//! ```
//! traits.rs  - BaseBackend + LLMBackend trait 定义（核心抽象）
//! types.rs  - BackendType, ResourceType, FinishReason 等类型
//! error.rs  - BackendError 错误类型
//! backend.rs    - 保留 InferenceBackend（向后兼容）
//! factory.rs - 后端工厂（创建后端实例）
//! runner/    - llama.cpp 通用推理抽象
//!   ├── llama_hardware.rs  - LlamaHardware trait
//!   └── sample.rs          - 采样算法
//! cpu.rs    - llama.cpp CPU 后端实现
//! cuda.rs   - llama.cpp CUDA 后端实现
//! ```

pub mod backend;
pub mod cpu;
pub mod error;
pub mod factory;
pub mod manager;
pub mod runner;
pub mod traits;
pub mod types;

#[cfg(feature = "rknn3")]
pub mod runner_rknn3;

#[cfg(feature = "cuda")]
pub mod cuda;

// Re-export for convenience
pub use backend::{InferenceBackend, InferenceConfig, ForwardResult, StreamToken, MemoryStats};
pub use error::BackendError;
pub use factory::BackendFactory;
pub use manager::BackendManager;
pub use traits::{BaseBackend, LLMBackend, LLMInferenceInput, LLMInferenceOptions, StreamChunk, LLMSteamChunkType, DroppableReceiver};
pub use types::{BackendType, FinishReason, ResourceType};