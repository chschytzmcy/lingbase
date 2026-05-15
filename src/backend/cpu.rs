//! CPU backend implementation using llama.cpp.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::pin::Pin;
use futures::Stream;
use tokio_stream::wrappers::ReceiverStream;
use crate::error::InferenceResult;
use crate::llama::{LlamaModel, LlamaContext, Tokenizer, batch::BatchWithTokens};
use super::{InferenceBackend, MemoryStats, InferenceConfig, ForwardResult, StreamToken};

pub struct CpuBackend {
    model: Option<LlamaModel>,
    context: Arc<Mutex<Option<LlamaContext>>>,
    tokenizer: Arc<Mutex<Option<Tokenizer>>>,
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
            context: Arc::new(Mutex::new(Some(context))),
            tokenizer: Arc::new(Mutex::new(Some(tokenizer))),
            n_ctx: n_ctx as u32,
            n_vocab,
        })
    }

    pub fn is_initialized(&self) -> bool {
        self.model.as_ref().map(|m| m.is_loaded()).unwrap_or(false)
            && self.context.lock().unwrap().is_some()
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

        let ctx_guard = self.context.lock().unwrap();
        let ctx = ctx_guard.as_ref().ok_or(crate::error::InferenceError::BackendNotInitialized)?;

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
            let tokenizer_guard = self.tokenizer.lock().unwrap();
            if tokenizer_guard.as_ref().map(|t| t.is_eog(sampled_token)).unwrap_or(false) {
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

    fn forward_stream(
        &self,
        tokens: &[i32],
        config: &InferenceConfig,
    ) -> Pin<Box<dyn Stream<Item = InferenceResult<StreamToken>> + Send>> {
        let (tx, rx) = tokio::sync::mpsc::channel::<InferenceResult<StreamToken>>(16);

        // Clone Arc references for the blocking task
        let context = Arc::clone(&self.context);
        let tokenizer = Arc::clone(&self.tokenizer);
        let n_vocab = self.n_vocab;
        let _n_ctx = self.n_ctx;
        let config = config.clone();
        let input_tokens = tokens.to_vec();

        tokio::task::spawn_blocking(move || {
            let start = Instant::now();

            let ctx_guard = context.lock().unwrap();
            let ctx = ctx_guard.as_ref();

            if ctx.is_none() {
                let _ = tx.blocking_send(Err(crate::error::InferenceError::BackendNotInitialized));
                return;
            }
            let ctx = ctx.unwrap();

            // Prefill phase
            let batch = match BatchWithTokens::new(&input_tokens) {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.blocking_send(Err(e));
                    return;
                }
            };
            if let Err(e) = ctx.decode(batch.batch) {
                let _ = tx.blocking_send(Err(e));
                return;
            }

            // Get logits and sample first token
            let logits = match ctx.get_logits(n_vocab) {
                Some(l) => l,
                None => {
                    let _ = tx.blocking_send(Err(crate::error::InferenceError::InferenceFailed(
                        "Failed to get logits".to_string()
                    )));
                    return;
                }
            };

            let _first_token_ms = start.elapsed().as_millis() as u64;
            let sampled_token = sample_token_impl(logits, &config, n_vocab);

            // Send first token
            {
                let tokenizer_guard = tokenizer.lock().unwrap();
                let text = tokenizer_guard.as_ref()
                    .and_then(|t| t.decode_tokens(&[sampled_token], true).ok())
                    .unwrap_or_default();

                let _ = tx.blocking_send(Ok(StreamToken {
                    token: sampled_token,
                    text,
                    is_first: true,
                    is_done: false,
                }));
            }

            // Generation loop
            let mut prev_token = sampled_token;
            for _ in 1..config.max_tokens {
                // Check EOS
                {
                    let tokenizer_guard = tokenizer.lock().unwrap();
                    if tokenizer_guard.as_ref().map(|t| t.is_eog(prev_token)).unwrap_or(false) {
                        let _ = tx.blocking_send(Ok(StreamToken {
                            token: 0,
                            text: String::new(),
                            is_first: false,
                            is_done: true,
                        }));
                        return;
                    }
                }

                // Decode next token
                let batch = match BatchWithTokens::new(&[prev_token]) {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(e));
                        return;
                    }
                };
                if let Err(e) = ctx.decode(batch.batch) {
                    let _ = tx.blocking_send(Err(e));
                    return;
                }

                let logits = match ctx.get_logits(n_vocab) {
                    Some(l) => l,
                    None => {
                        let _ = tx.blocking_send(Err(crate::error::InferenceError::InferenceFailed(
                            "Failed to get logits".to_string()
                        )));
                        return;
                    }
                };

                let next_token = sample_token_impl(logits, &config, n_vocab);

                // Send token
                {
                    let tokenizer_guard = tokenizer.lock().unwrap();
                    let text = tokenizer_guard.as_ref()
                        .and_then(|t| t.decode_tokens(&[next_token], true).ok())
                        .unwrap_or_default();

                    let _ = tx.blocking_send(Ok(StreamToken {
                        token: next_token,
                        text,
                        is_first: false,
                        is_done: false,
                    }));
                }

                prev_token = next_token;
            }

            // Send done signal
            let _ = tx.blocking_send(Ok(StreamToken {
                token: 0,
                text: String::new(),
                is_first: false,
                is_done: true,
            }));
        });

        Box::pin(ReceiverStream::new(rx))
    }

    fn max_context_size(&self) -> usize {
        self.n_ctx as usize
    }

    fn sample_token(&self, logits: &[f32], config: &InferenceConfig) -> i32 {
        sample_token_impl(logits, config, self.n_vocab)
    }

    fn tokenize(&self, text: &str) -> InferenceResult<Vec<i32>> {
        let tokenizer_guard = self.tokenizer.lock().unwrap();
        let tokenizer = tokenizer_guard.as_ref()
            .ok_or(crate::error::InferenceError::BackendNotInitialized)?;
        tokenizer.encode(text, true)
    }

    fn detokenize(&self, tokens: &[i32]) -> InferenceResult<String> {
        let tokenizer_guard = self.tokenizer.lock().unwrap();
        let tokenizer = tokenizer_guard.as_ref()
            .ok_or(crate::error::InferenceError::BackendNotInitialized)?;
        tokenizer.decode_tokens(tokens, true)
    }
}

fn sample_token_impl(logits: &[f32], config: &InferenceConfig, n_vocab: usize) -> i32 {
    let temperature = config.temperature;
    let top_p = config.top_p;
    let top_k = config.top_k;

    // Find top-k candidates first
    let mut indices: Vec<usize> = (0..n_vocab).collect();

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
    for (i, &_idx) in indices.iter().enumerate() {
        if probs[i] > max_val {
            max_val = probs[i];
            max_idx = i;
        }
    }

    // Return the actual token ID from the sorted indices array
    indices[max_idx] as i32
}