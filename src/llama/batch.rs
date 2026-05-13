//! Batch management for llama.cpp

use std::ptr;
use crate::error::InferenceResult;
use super::context::llama_token;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct llama_batch {
    pub n_tokens: i32,
    pub token: *mut llama_token,
    pub pos: *mut i32,
    pub n_seq_id: *mut i32,
    pub seq_id: *mut *mut i32,
    pub logits: *mut bool,
}

impl llama_batch {
    pub fn new(n_tokens: usize) -> InferenceResult<Self> {
        let token = vec![0i32; n_tokens].into_boxed_slice();
        let pos = vec![0i32; n_tokens].into_boxed_slice();
        let n_seq_id = vec![0i32; n_tokens].into_boxed_slice();
        let mut seq_id = vec![ptr::null_mut::<i32>(); n_tokens];
        let mut logits = vec![false; n_tokens];

        Ok(Self {
            n_tokens: 0,
            token: Box::into_raw(token) as *mut llama_token,
            pos: Box::into_raw(pos) as *mut i32,
            n_seq_id: Box::into_raw(n_seq_id) as *mut i32,
            seq_id: seq_id.as_mut_ptr(),
            logits: logits.as_mut_ptr(),
        })
    }

    pub fn add(&mut self, token: llama_token, pos: i32, seq_id: i32) {
        let idx = self.n_tokens as usize;
        unsafe {
            *self.token.add(idx) = token;
            *self.pos.add(idx) = pos;
            *self.n_seq_id.add(idx) = 1;
            *self.seq_id.add(idx) = &seq_id as *const i32 as *mut i32;
            *self.logits.add(idx) = true;
        }
        self.n_tokens += 1;
    }

    pub fn n_tokens(&self) -> i32 {
        self.n_tokens
    }
}

unsafe impl Send for llama_batch {}
unsafe impl Sync for llama_batch {}

extern "C" {
    fn llama_decode_batch(ctx: *mut libc::c_void, batch: *mut llama_batch) -> i32;
    fn llama_eval_internal(ctx: *mut libc::c_void, tokens: *mut llama_token, n_tokens: i32, n_past: i32) -> i32;
}

pub fn llama_decode(_ctx: *mut libc::c_void, _batch: *mut llama_batch) -> InferenceResult<()> {
    // TODO: llama.cpp not compiled
    Err(crate::error::InferenceError::BackendNotInitialized)
}

pub fn llama_eval(_ctx: *mut libc::c_void, _tokens: &mut [llama_token], _n_past: i32) -> InferenceResult<()> {
    // TODO: llama.cpp not compiled
    Err(crate::error::InferenceError::BackendNotInitialized)
}