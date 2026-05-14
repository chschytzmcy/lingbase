//! Tokenization utilities using llama.cpp vocab.

use std::ffi::CString;
use tracing::debug;
use crate::error::{InferenceError, InferenceResult};
use super::ffi::{VocabPtr, llama_tokenize, llama_vocab_bos, llama_vocab_eos, llama_vocab_is_eog, llama_detokenize};

pub struct Tokenizer {
    vocab: VocabPtr,
}

unsafe impl Send for Tokenizer {}
unsafe impl Sync for Tokenizer {}

impl Tokenizer {
    pub fn new(vocab: VocabPtr) -> Self {
        Self { vocab }
    }

    pub fn encode(&self, text: &str, add_special: bool) -> InferenceResult<Vec<i32>> {
        if self.vocab.is_null() {
            return Err(InferenceError::BackendNotInitialized);
        }

        let text_c = CString::new(text)
            .map_err(|e| InferenceError::TokenizationFailed(e.to_string()))?;

        let mut tokens = vec![0i32; 8192];
        let n = unsafe {
            llama_tokenize(
                self.vocab,
                text_c.as_ptr(),
                text.len() as i32,
                tokens.as_mut_ptr(),
                tokens.len() as i32,
                add_special,
                true,  // parse_special=true: convert special token text to special token IDs
            )
        };

        if n < 0 {
            return Err(InferenceError::TokenizationFailed(
                format!("Tokenization failed, need {} tokens", -n)
            ));
        }

        tokens.truncate(n as usize);
        Ok(tokens)
    }

    pub fn decode(&self, token: i32, remove_special: bool) -> InferenceResult<String> {
        self.decode_tokens(&[token], remove_special)
    }

    pub fn decode_tokens(&self, tokens: &[i32], remove_special: bool) -> InferenceResult<String> {
        self.decode_tokens_full(tokens, remove_special, false)
    }

    pub fn decode_tokens_full(&self, tokens: &[i32], remove_special: bool, unparse_special: bool) -> InferenceResult<String> {
        if self.vocab.is_null() {
            return Err(InferenceError::BackendNotInitialized);
        }

        if tokens.is_empty() {
            return Ok(String::new());
        }

        // For BPE vocab (type 2), manually filter special tokens and decode the rest
        if remove_special {
            let eos_token = unsafe { llama_vocab_eos(self.vocab) };
            let bos_token = unsafe { llama_vocab_bos(self.vocab) };

            // Debug: log token IDs
            debug!("Detokenize: input tokens {:?}", tokens);
            debug!("Detokenize: EOS={}, BOS={}", eos_token, bos_token);

            // Filter out EOS/BOS and tokens that are special control tokens (high ID range)
            // Qwen3 special tokens are in range 151643-151667 (im_start=151644, im_end=151645, etc.)
            // These are typically at the end of vocabulary and have high token IDs
            let filtered: Vec<i32> = tokens.iter()
                .filter(|&&t| {
                    // Skip explicit EOS/BOS tokens
                    if t == eos_token || t == bos_token {
                        debug!("Filtering out EOS/BOS token {}", t);
                        return false;
                    }
                    // Skip tokens that are end-of-generation (use llama's built-in check)
                    if unsafe { llama_vocab_is_eog(self.vocab, t) } {
                        debug!("Filtering out is_eog token {}", t);
                        return false;
                    }
                    // For Qwen models, special control tokens have IDs >= 151643
                    // Filter out high-ID tokens that are likely special/control tokens
                    if t >= 151643 {
                        debug!("Filtering out high-ID token {}", t);
                        return false;
                    }
                    true
                })
                .cloned()
                .collect();

            debug!("Detokenize: filtered tokens {:?}", filtered);

            if filtered.is_empty() {
                return Ok(String::new());
            }

            // Decode filtered tokens without special token processing
            return self.decode_tokens_full(&filtered, false, false);
        }

        let mut buf = vec![0u8; 16384];

        let n = unsafe {
            llama_detokenize(
                self.vocab,
                tokens.as_ptr(),
                tokens.len() as i32,
                buf.as_mut_ptr() as *mut i8,
                buf.len() as i32,
                remove_special,
                unparse_special,
            )
        };

        if n < 0 {
            return Err(InferenceError::TokenizationFailed("Detokenization failed".to_string()));
        }

        Ok(String::from_utf8_lossy(&buf[..n as usize]).to_string())
    }

    pub fn bos_token(&self) -> i32 {
        if self.vocab.is_null() { -1 } else { unsafe { llama_vocab_bos(self.vocab) } }
    }

    pub fn eos_token(&self) -> i32 {
        if self.vocab.is_null() { -1 } else { unsafe { llama_vocab_eos(self.vocab) } }
    }

    pub fn is_eog(&self, token: i32) -> bool {
        if self.vocab.is_null() { false } else { unsafe { llama_vocab_is_eog(self.vocab, token) } }
    }
}