//! InferenceBackend trait definition.

use crate::error::InferenceResult;

#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct InferenceConfig {
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: i32,
    pub repeat_penalty: f32,
    pub timeout_ms: Option<u64>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            repeat_penalty: 1.1,
            timeout_ms: Some(60000),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ForwardResult {
    pub tokens: Vec<i32>,
    pub first_token_ms: Option<u64>,
    pub total_ms: u64,
}

pub trait InferenceBackend: Send + Sync {
    fn name(&self) -> &str;
    fn health_check(&self) -> bool;
    fn memory_stats(&self) -> MemoryStats;
    fn forward(&self, tokens: &[i32], config: &InferenceConfig) -> InferenceResult<ForwardResult>;
    fn max_context_size(&self) -> usize;
    fn sample_token(&self, logits: &[f32], config: &InferenceConfig) -> i32;
    fn tokenize(&self, text: &str) -> InferenceResult<Vec<i32>>;
    fn detokenize(&self, tokens: &[i32]) -> InferenceResult<String>;
}