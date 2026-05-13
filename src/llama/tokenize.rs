//! Tokenization utilities.

use crate::error::{InferenceError, InferenceResult};
use super::context::llama_token;

extern "C" {
    fn llama_tokenize(
        ctx: *mut libc::c_void,
        text: *const libc::c_char,
        tokens: *mut llama_token,
        n_max_tokens: i32,
        add_bos: bool,
    ) -> i32;

    fn llama_token_to_str(ctx: *mut libc::c_void, token: llama_token) -> *const libc::c_char;
    fn llama_token_eos() -> llama_token;
    fn llama_token_bos() -> llama_token;
}

pub struct Tokenizer {
    initialized: bool,
}

impl Tokenizer {
    pub fn new(_ctx: *mut libc::c_void) -> Self {
        Self { initialized: false }
    }

    pub fn encode(&self, _text: &str, _add_bos: bool) -> InferenceResult<Vec<llama_token>> {
        Err(InferenceError::BackendNotInitialized)
    }

    pub fn decode(&self, _token: llama_token) -> InferenceResult<String> {
        Err(InferenceError::BackendNotInitialized)
    }

    pub fn eos() -> llama_token {
        unsafe { llama_token_eos() }
    }

    pub fn bos() -> llama_token {
        unsafe { llama_token_bos() }
    }
}