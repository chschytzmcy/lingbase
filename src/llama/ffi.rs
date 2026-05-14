//! FFI bindings to llama.cpp C API.
//!
//! This module defines the unsafe FFI interface to the llama.cpp C library.

use std::ffi::c_char;
use libc::size_t;

pub type LlamaToken = i32;
pub type LlamaPos = i32;
pub type LlamaSeqId = i32;

// Opaque pointer types - raw pointers are Send + Sync when wrapped properly
pub type ModelPtr = *mut libc::c_void;
pub type ContextPtr = *mut libc::c_void;
pub type VocabPtr = *mut libc::c_void;

// Forward declarations for structs we don't need to inspect in Rust
#[repr(C)]
pub struct LlamaModelParams {
    pub devices: *mut libc::c_void,
    pub tensor_buft_overrides: *const libc::c_void,
    pub n_gpu_layers: i32,
    pub split_mode: i32,
    pub main_gpu: i32,
    pub tensor_split: *const f32,
    pub progress_callback: Option<unsafe extern "C" fn(f32, *mut libc::c_void)>,
    pub progress_callback_user_data: *mut libc::c_void,
    pub kv_overrides: *const libc::c_void,
    pub vocab_only: bool,
    pub use_mmap: bool,
    pub use_direct_io: bool,
    pub use_mlock: bool,
    pub check_tensors: bool,
    pub use_extra_bufts: bool,
    pub no_host: bool,
    pub no_alloc: bool,
}

#[repr(C)]
#[derive(Default)]
pub struct LlamaContextParams {
    pub n_ctx: u32,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub n_seq_max: u32,
    pub n_threads: i32,
    pub n_threads_batch: i32,
    pub rope_scaling_type: i32,
    pub pooling_type: i32,
    pub attention_type: i32,
    pub flash_attn_type: i32,
    pub rope_freq_base: f32,
    pub rope_freq_scale: f32,
    pub yarn_ext_factor: f32,
    pub yarn_attn_factor: f32,
    pub yarn_beta_fast: f32,
    pub yarn_beta_slow: f32,
    pub yarn_orig_ctx: u32,
    pub defrag_thold: f32,
    pub cb_eval: *mut libc::c_void,
    pub cb_eval_user_data: *mut libc::c_void,
    pub type_k: i32,
    pub type_v: i32,
    pub abort_callback: Option<unsafe extern "C" fn() -> bool>,
    pub abort_callback_data: *mut libc::c_void,
    pub embeddings: bool,
    pub offload_kqv: bool,
    pub no_perf: bool,
    pub op_offload: bool,
    pub swa_full: bool,
    pub kv_unified: bool,
    pub samplers: *mut libc::c_void,
    pub n_samplers: size_t,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct LlamaBatch {
    pub n_tokens: i32,
    pub token: *mut LlamaToken,
    pub embd: *mut f32,
    pub pos: *mut LlamaPos,
    pub n_seq_id: *mut i32,
    pub seq_id: *mut *mut LlamaSeqId,
    pub logits: *mut i8,
}

// extern "C" declarations
extern "C" {
    // Backend initialization
    pub fn llama_backend_init();
    pub fn ggml_backend_load_all();
    pub fn ggml_backend_load_all_from_path(path: *const c_char);

    // Model functions
    pub fn llama_model_default_params() -> LlamaModelParams;
    pub fn llama_context_default_params() -> LlamaContextParams;

    pub fn llama_model_load_from_file(path: *const c_char, params: LlamaModelParams) -> ModelPtr;
    pub fn llama_model_free(model: ModelPtr);

    pub fn llama_init_from_model(model: ModelPtr, params: LlamaContextParams) -> ContextPtr;
    pub fn llama_free(ctx: ContextPtr);

    pub fn llama_get_model(ctx: ContextPtr) -> ModelPtr;
    pub fn llama_n_ctx(ctx: ContextPtr) -> u32;

    pub fn llama_model_get_vocab(model: ModelPtr) -> VocabPtr;
    pub fn llama_model_n_ctx_train(model: ModelPtr) -> i32;
    pub fn llama_model_n_embd(model: ModelPtr) -> i32;
    pub fn llama_model_n_vocab(model: ModelPtr) -> i32;

    // Decode functions - llama_decode takes batch by value
    pub fn llama_decode(ctx: ContextPtr, batch: LlamaBatch) -> i32;
    pub fn llama_encode(ctx: ContextPtr, batch: LlamaBatch) -> i32;

    // Logits
    pub fn llama_get_logits(ctx: ContextPtr) -> *mut f32;
    pub fn llama_get_logits_ith(ctx: ContextPtr, i: i32) -> *mut f32;

    // Vocab functions
    pub fn llama_vocab_n_tokens(vocab: VocabPtr) -> i32;
    pub fn llama_vocab_get_text(vocab: VocabPtr, token: LlamaToken) -> *const c_char;
    pub fn llama_vocab_get_score(vocab: VocabPtr, token: LlamaToken) -> f32;
    pub fn llama_vocab_is_eog(vocab: VocabPtr, token: LlamaToken) -> bool;
    pub fn llama_vocab_bos(vocab: VocabPtr) -> LlamaToken;
    pub fn llama_vocab_eos(vocab: VocabPtr) -> LlamaToken;
    pub fn llama_vocab_type(vocab: VocabPtr) -> i32;

    // Tokenization
    pub fn llama_tokenize(
        vocab: VocabPtr,
        text: *const c_char,
        text_len: i32,
        tokens: *mut LlamaToken,
        n_tokens_max: i32,
        add_special: bool,
        parse_special: bool,
    ) -> i32;

    pub fn llama_detokenize(
        vocab: VocabPtr,
        tokens: *const LlamaToken,
        n_tokens: i32,
        text: *mut c_char,
        text_len_max: i32,
        remove_special: bool,
        unparse_special: bool,
    ) -> i32;

    // Batch allocation - llama_batch_init returns batch by value
    pub fn llama_batch_init(n_tokens: i32, embd: i32, n_seq_max: i32) -> LlamaBatch;
    pub fn llama_batch_free(batch: LlamaBatch);

    // Use this instead of manual batch creation
    pub fn llama_batch_get_one(tokens: *mut LlamaToken, n_tokens: i32) -> LlamaBatch;

    // System info
    pub fn llama_print_system_info() -> *const c_char;
}