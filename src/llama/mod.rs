//! llama.cpp FFI module - wraps the C API for model loading and inference.

pub mod model;
pub mod context;
pub mod batch;
pub mod tokenize;

pub use model::LlamaModel;
pub use context::LlamaContext;
pub use batch::{llama_batch, llama_eval, llama_decode};
pub use tokenize::Tokenizer;