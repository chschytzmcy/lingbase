//! CPU backend implementation using llama.cpp.

use std::path::Path;
use std::time::Instant;
use crate::error::InferenceResult;
use crate::llama::{LlamaModel, LlamaContext, Tokenizer, batch::BatchWithTokens};
use super::{InferenceBackend, MemoryStats, InferenceConfig, ForwardResult};

pub struct CpuBackend {
    model: Option<LlamaModel>,
    context: Option<LlamaContext>,
    tokenizer: Option<Tokenizer>,
    n_ctx: u32,
    n_vocab: usize,
}

impl CpuBackend {
    pub fn new<P: AsRef<Path>>(model_path: P, n_ctx: i32) -> InferenceResult<Self> {
        let model = LlamaModel::from_file(model_path, 0)?;
        let model_ptr = model.ptr().ok_or(crate::error::InferenceError::ModelNotLoaded)?;
        let vocab_ptr = model.vocab_ptr().ok_or(crate::error::InferenceError::BackendNotInitialized)?;
        let n_vocab = model.n_vocab() as usize;

        let context = LlamaContext::new(model_ptr, n_ctx as u32, 4)?;
        let tokenizer = Tokenizer::new(vocab_ptr);

        Ok(Self {
            model: Some(model),
            context: Some(context),
            tokenizer: Some(tokenizer),
            n_ctx: n_ctx as u32,
            n_vocab,
        })
    }

    pub fn is_initialized(&self) -> bool {
        self.model.as_ref().map(|m| m.is_loaded()).unwrap_or(false)
            && self.context.is_some()
    }
}

impl InferenceBackend for CpuBackend {
    fn name(&self) -> &str {
        "cpu"
    }

    fn health_check(&self) -> bool {
        self.is_initialized()
    }

    fn memory_stats(&self) -> MemoryStats {
        MemoryStats {
            used_bytes: 0,
            total_bytes: 0,
            free_bytes: 0,
        }
    }

    fn forward(&self, tokens: &[i32], config: &InferenceConfig) -> InferenceResult<ForwardResult> {
        let start = Instant::now();

        let ctx = self.context.as_ref().ok_or(crate::error::InferenceError::BackendNotInitialized)?;

        // Prefill phase
        let mut batch = BatchWithTokens::new(tokens)?;
        ctx.decode(batch.batch)?;

        // Get logits and sample next token
        let logits = ctx.get_logits(self.n_vocab).ok_or(crate::error::InferenceError::InferenceFailed(
            "Failed to get logits".to_string()
        ))?;

        let first_token_ms = Some(start.elapsed().as_millis() as u64);
        let mut sampled_token = self.sample_token(logits, config);

        // Generation loop - reuse batch wrapper, just update tokens
        let mut output_tokens = vec![sampled_token];
        let max_new = config.max_tokens as i32;

        while output_tokens.len() < max_new as usize {
            if self.tokenizer.as_ref().map(|t| t.is_eog(sampled_token)).unwrap_or(false) {
                break;
            }

            batch = BatchWithTokens::new(&[sampled_token])?;
            ctx.decode(batch.batch)?;

            let logits = ctx.get_logits(self.n_vocab).ok_or(crate::error::InferenceError::InferenceFailed(
                "Failed to get logits".to_string()
            ))?;

            sampled_token = self.sample_token(logits, config);
            output_tokens.push(sampled_token);
        }

        let total_ms = start.elapsed().as_millis() as u64;

        Ok(ForwardResult {
            tokens: output_tokens,
            first_token_ms,
            total_ms,
        })
    }

    fn max_context_size(&self) -> usize {
        self.n_ctx as usize
    }

    fn sample_token(&self, logits: &[f32], config: &InferenceConfig) -> i32 {
        let temperature = config.temperature;
        let top_p = config.top_p;
        let top_k = config.top_k;

        // Find top-k candidates first
        let mut indices: Vec<usize> = (0..logits.len()).collect();

        if top_k > 0 {
            indices.sort_by(|&a, &b| logits[b].partial_cmp(&logits[a]).unwrap());
            indices.truncate(top_k as usize);
        } else {
            indices.sort_by(|&a, &b| logits[b].partial_cmp(&logits[a]).unwrap());
        }

        // Apply temperature and compute probabilities
        let mut probs: Vec<f32> = indices.iter()
            .map(|&i| {
                let logit = logits[i];
                if temperature > 0.0 && temperature != 1.0 {
                    logit / temperature
                } else {
                    logit
                }
            })
            .collect();

        // Compute softmax
        let max_logit = probs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_sum: f32 = probs.iter().map(|&l| (l - max_logit).exp()).sum();

        if top_p < 1.0 && !probs.is_empty() {
            // Top-p (nucleus) sampling
            let mut cumsum = 0.0f32;
            for (i, prob) in probs.iter_mut().enumerate() {
                let p = (*prob - max_logit).exp() / exp_sum;
                cumsum += p;
                if cumsum > top_p {
                    // Zero out remaining probabilities
                    for j in i..probs.len() {
                        probs[j] = f32::NEG_INFINITY;
                    }
                    break;
                }
            }
        }

        // Find the selected index - probs corresponds to top-k candidates in indices
        let mut max_idx = 0;
        let mut max_val = f32::NEG_INFINITY;
        for (i, &idx) in indices.iter().enumerate() {
            if probs[i] > max_val {
                max_val = probs[i];
                max_idx = i;
            }
        }

        // Return the actual token ID from the sorted indices array
        indices[max_idx] as i32
    }

    fn tokenize(&self, text: &str) -> InferenceResult<Vec<i32>> {
        let tokenizer = self.tokenizer.as_ref()
            .ok_or(crate::error::InferenceError::BackendNotInitialized)?;
        tokenizer.encode(text, true)
    }

    fn detokenize(&self, tokens: &[i32]) -> InferenceResult<String> {
        let tokenizer = self.tokenizer.as_ref()
            .ok_or(crate::error::InferenceError::BackendNotInitialized)?;
        tokenizer.decode_tokens(tokens, true)
    }
}