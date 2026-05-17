//! rknn3-cli — CLI for running LLM inference via RKNN3 Runtime.
//!
//! Usage:
//!   rknn3-cli --model model.rknn --weight weight.rknn \
//!     --tokenizer model.tokenizer.gguf --embedding embed.bin \
//!     --prompt "Hello"
//!
//!   rknn3-cli --model model.rknn --weight weight.rknn \
//!     --tokenizer model.tokenizer.gguf --embedding embed.bin \
//!     --prompt "Explain quantum computing" --think

use std::fs;
use std::io::Write;
use std::time::Instant;

use anyhow::{Context as _, Result};
use clap::Parser;
use rknn3_sys::prelude::{
    CallbackError, Context, InferParams, LlmCallState, LlmCallbacks, LlmInput, LlmParams,
    ModelConfig, Session,
};
use shimmytok::byte_encoder;

/// RKNN3 LLM inference CLI
#[derive(Parser, Debug)]
#[command(
    name = "rknn3-cli",
    version,
    about = "Run LLM inference via RKNN3 Runtime"
)]
struct Cli {
    /// Path to the .rknn model file
    #[arg(long)]
    model: String,

    /// Path to the weight file
    #[arg(long)]
    weight: String,

    /// Path to GGUF tokenizer file (model.tokenizer.gguf)
    #[arg(long)]
    tokenizer: String,

    /// Path to embedding file (.embed.bin)
    #[arg(long)]
    embedding: String,

    /// Text prompt
    #[arg(long)]
    prompt: String,

    /// Maximum context length
    #[arg(long, default_value_t = 4096)]
    context_len: i32,

    /// Maximum new tokens to generate
    #[arg(long, default_value_t = 512)]
    max_tokens: i32,

    /// Sampling temperature (0.0 = greedy)
    #[arg(long, default_value_t = 1.0)]
    temperature: f32,

    /// Top-K sampling
    #[arg(long, default_value_t = 1)]
    top_k: i32,

    /// Top-P (nucleus) sampling
    #[arg(long, default_value_t = 0.9)]
    top_p: f32,

    /// Repeat penalty
    #[arg(long, default_value_t = 1.2)]
    repeat_penalty: f32,

    /// Vocab size
    #[arg(long, default_value_t = 151936)]
    vocab_size: i32,

    /// NPU core mask (hex, e.g. 0xff for all cores)
    #[arg(long, default_value = "0xff")]
    core_mask: String,

    /// Enable thinking mode (for thinking models like Qwen3)
    #[arg(long, default_value_t = false)]
    think: bool,

    /// Verbosity level: -v = token IDs, -vv = token IDs + decoded pieces
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

// ---------------------------------------------------------------------------
// Embedding (float16 lookup table)
// ---------------------------------------------------------------------------

struct Embedding {
    data: Vec<u8>,
    vocab_size: usize,
    embedding_dim: usize,
}

impl Embedding {
    fn from_file(path: &str, vocab_size: usize) -> Result<Self> {
        let data =
            fs::read(path).with_context(|| format!("Failed to read embedding file: {path}"))?;
        let embedding_dim = (data.len() / vocab_size) / 2;
        eprintln!(
            "[rknn3-cli] Embedding loaded: {} bytes, dim={}",
            data.len(),
            embedding_dim
        );
        Ok(Self {
            data,
            vocab_size,
            embedding_dim,
        })
    }
}

// ---------------------------------------------------------------------------
// Byte-buffered decoder for streaming token output
// ---------------------------------------------------------------------------
//
// RKNN3 calls on_result per-token (not in batches). Emoji UTF-8 bytes are split
// across multiple tokens. We accumulate raw bytes from each token's vocab text
// (GPT-2 byte-encoded) and flush complete UTF-8 characters as they become available.

struct ByteBufferedDecoder {
    buf: Vec<u8>,
}

impl ByteBufferedDecoder {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Decode GPT-2 byte-encoded text to raw bytes (no UTF-8 lossy conversion).
    fn piece_to_raw_bytes(piece: &str) -> Vec<u8> {
        let decoder = byte_encoder::unicode_to_bytes();
        piece
            .chars()
            .filter_map(|c| decoder.get(&c).copied())
            .collect()
    }

    /// Feed token vocab texts (byte-encoded) into the buffer, return decoded output.
    fn feed(&mut self, pieces: &[&str]) -> String {
        for piece in pieces {
            let raw = Self::piece_to_raw_bytes(piece);
            self.buf.extend_from_slice(&raw);
        }
        self.drain_utf8()
    }

    /// Flush any remaining bytes (may produce replacement chars for incomplete UTF-8).
    fn flush(&mut self) -> String {
        let result = String::from_utf8_lossy(&self.buf).into_owned();
        self.buf.clear();
        result
    }

    /// Extract complete UTF-8 characters from the buffer, leaving incomplete bytes.
    fn drain_utf8(&mut self) -> String {
        match std::str::from_utf8(&self.buf) {
            Ok(_) => {
                let s = std::mem::take(&mut self.buf);
                String::from_utf8(s).unwrap_or_default()
            }
            Err(e) => {
                let valid_up_to = e.valid_up_to();
                if valid_up_to == 0 {
                    let keep = self.buf.len().min(3);
                    let drain = self.buf.len() - keep;
                    let valid_bytes: Vec<u8> = self.buf[..drain].to_vec();
                    self.buf.drain(..drain);
                    String::from_utf8_lossy(&valid_bytes).into_owned()
                } else {
                    let valid_bytes: Vec<u8> = self.buf[..valid_up_to].to_vec();
                    self.buf.drain(..valid_up_to);
                    String::from_utf8(valid_bytes).unwrap_or_default()
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CliCallbacks — safe callback implementation via LlmCallbacks trait
// ---------------------------------------------------------------------------

struct CliCallbacks {
    tokenizer: shimmytok::Tokenizer,
    embedding: Embedding,
    decoder: ByteBufferedDecoder,
    verbose: u8,
}

impl LlmCallbacks for CliCallbacks {
    fn tokenize(&self, text: &str, buf: &mut [i32]) -> Result<usize, CallbackError> {
        match self.tokenizer.encode(text, false) {
            Ok(ids) => {
                let len = ids.len().min(buf.len());
                for (i, &id) in ids.iter().take(len).enumerate() {
                    buf[i] = id as i32;
                }
                Ok(len)
            }
            Err(_) => Err(CallbackError::TokenizeFailed),
        }
    }

    fn embed(&self, tokens: &[i32], buf: &mut [u8]) -> Result<(), CallbackError> {
        let expected_len = tokens.len() * self.embedding.embedding_dim * 2;
        if buf.len() != expected_len {
            return Err(CallbackError::InvalidInput);
        }
        for (n, &token_id) in tokens.iter().enumerate() {
            if token_id < 0 || (token_id as usize) >= self.embedding.vocab_size {
                return Err(CallbackError::InvalidInput);
            }
            let src_offset = token_id as usize * self.embedding.embedding_dim * 2;
            let dst_offset = n * self.embedding.embedding_dim * 2;
            let copy_len = self.embedding.embedding_dim * 2;
            buf[dst_offset..dst_offset + copy_len]
                .copy_from_slice(&self.embedding.data[src_offset..src_offset + copy_len]);
        }
        Ok(())
    }

    fn on_result(&mut self, token_ids: &[i32], state: LlmCallState) {
        match state {
            LlmCallState::Normal => {
                if self.verbose >= 1 {
                    eprintln!("\n[rknn3-cli] [Normal] token_ids={:?}", token_ids);
                }
                // Collect vocab text for each non-special token
                let pieces: Vec<String> = token_ids
                    .iter()
                    .filter(|&&tid| !self.tokenizer.is_special_token(tid as u32))
                    .filter_map(|&tid| self.tokenizer.token_to_piece(tid as u32).ok())
                    .collect();
                if pieces.is_empty() {
                    return;
                }
                if self.verbose >= 2 {
                    eprintln!("[rknn3-cli] [Normal] pieces={:?}", pieces);
                }
                let refs: Vec<&str> = pieces.iter().map(String::as_str).collect();
                let text = self.decoder.feed(&refs);
                print!("{text}");
                std::io::stdout().flush().unwrap();
            }
            LlmCallState::Finish => {
                if self.verbose >= 1 {
                    eprintln!("\n[rknn3-cli] [Finish] inference complete");
                }
                let remaining = self.decoder.flush();
                if !remaining.is_empty() {
                    print!("{remaining}");
                    std::io::stdout().flush().unwrap();
                }
                println!();
            }
            LlmCallState::Error => {
                eprintln!("\n[rknn3-cli] Inference error");
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();
    let core_mask = u32::from_str_radix(cli.core_mask.trim_start_matches("0x"), 16).unwrap_or(0xff);

    eprintln!("[rknn3-cli] Model: {}", cli.model);
    eprintln!("[rknn3-cli] Weight: {}", cli.weight);
    eprintln!("[rknn3-cli] Tokenizer: {}", cli.tokenizer);
    eprintln!("[rknn3-cli] Embedding: {}", cli.embedding);
    eprintln!("[rknn3-cli] Core mask: 0x{:x}", core_mask);

    // 1. Initialize RKNN3 context
    let ctx = Context::new().map_err(|e| anyhow::anyhow!("rknn3_init failed: {e}"))?;
    eprintln!("[rknn3-cli] Context initialized.");

    // 2. Load model
    ctx.load_model(&cli.model, &cli.weight)
        .map_err(|e| anyhow::anyhow!("rknn3_load_model_from_path failed: {e}"))?;
    eprintln!("[rknn3-cli] Model loaded.");

    // 3. Initialize model
    let mut config = ModelConfig::new().core_mask(core_mask);
    ctx.model_init(&mut config)
        .map_err(|e| anyhow::anyhow!("rknn3_model_init failed: {e}"))?;
    eprintln!("[rknn3-cli] Model initialized.");

    // 3.5 Query LLM config
    let llm_cfg = ctx
        .query_llm_config()
        .map_err(|e| anyhow::anyhow!("rknn3_query LLM config failed: {e}"))?;
    eprintln!("[rknn3-cli] LLM config:");
    eprintln!(
        "  vocab_size={}, embedding_dim={}, max_ctx_len={}, max_position_embeddings={}",
        llm_cfg.vocab_size,
        llm_cfg.embedding_dim,
        llm_cfg.max_ctx_len,
        llm_cfg.max_position_embeddings
    );
    eprintln!(
        "  kvcache_dtype={:?}, kvcache_group_size={}, kvcache_residual_depth={}",
        llm_cfg.kvcache_dtype, llm_cfg.kvcache_group_size, llm_cfg.kvcache_residual_depth
    );
    if let Some(ref model_type) = llm_cfg.model_type {
        eprintln!("  model_type={}", model_type);
    }
    eprintln!("  task_type={:?}", llm_cfg.task_type);

    // 3.6 Validate context_len against max_ctx_len
    let context_len = cli.context_len as u32;
    if context_len > llm_cfg.max_ctx_len {
        anyhow::bail!(
            "context_len ({}) exceeds max_ctx_len ({}). \
             Please reduce --context-len or re-export the model with a larger KV cache.",
            context_len,
            llm_cfg.max_ctx_len
        );
    }
    eprintln!(
        "[rknn3-cli] context_len={} <= max_ctx_len={} ✓",
        context_len, llm_cfg.max_ctx_len
    );

    // 4. Create LLM session using safe builder pattern
    let eos_ids: [i32; 5] = [151643, 151645, 151662, 151663, 151664];

    let llm_param = LlmParams::new("logits", cli.vocab_size)
        .map_err(|e| anyhow::anyhow!("LlmParams::new failed: {e}"))?
        .max_context_len(cli.context_len)
        .top_k(cli.top_k)
        .top_p(cli.top_p)
        .temperature(cli.temperature)
        .repeat_penalty(cli.repeat_penalty)
        .special_bos_id(&[151643])
        .special_eos_id(&eos_ids)
        .linefeed_id(198);

    let mut session = Session::new(&ctx, &mut [llm_param])
        .map_err(|e| anyhow::anyhow!("rknn3_session_init failed: {e}"))?;
    eprintln!("[rknn3-cli] Session created.");

    // 5. Load tokenizer and embedding
    let tokenizer = shimmytok::Tokenizer::from_gguf_file(&cli.tokenizer)
        .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {e}"))?;
    eprintln!(
        "[rknn3-cli] Tokenizer loaded, vocab_size={}",
        tokenizer.vocab_size()
    );

    let embedding = Embedding::from_file(&cli.embedding, cli.vocab_size as usize)?;

    // 6. Build Qwen3 chat template manually in token mode to control thinking.
    //
    // Encode the base prompt first, then manually append think suppression tokens
    // (151667 = imediate, 151668 = imediate) when thinking is disabled.
    // This avoids relying on RKNN3 Runtime's internal chat template handling.
    let base_prompt = format!(
        "<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
        cli.prompt
    );

    let mut token_ids: Vec<i32> = tokenizer
        .encode(&base_prompt, false)
        .map_err(|e| anyhow::anyhow!("Failed to encode chat prompt: {e}"))?
        .into_iter()
        .map(|id| id as i32)
        .collect();

    if !cli.think {
        // Append think suppression: imediate + newline + newline + imediate + newline + newline
        // Token IDs: 151667 ( imediate), 151668 ( imediate), 198 (linefeed)
        token_ids.push(151667); // imediate
        token_ids.push(198); // \n
        token_ids.push(198); // \n
        token_ids.push(151668); // imediate
        token_ids.push(198); // \n
        token_ids.push(198); // \n
    }

    // 7. Set callbacks (tokenizer is moved here)
    let callbacks = CliCallbacks {
        tokenizer,
        embedding,
        decoder: ByteBufferedDecoder::new(),
        verbose: cli.verbose,
    };
    session
        .set_callback(Box::new(callbacks))
        .map_err(|e| anyhow::anyhow!("rknn3_session_set_callback failed: {e}"))?;
    eprintln!("[rknn3-cli] Callbacks set successfully.");

    // 8. Run inference
    eprintln!();
    eprintln!("User: {}", cli.prompt);
    eprint!("Qwen: ");
    std::io::stderr().flush().unwrap();

    let llm_input = LlmInput::tokens(token_ids).role("user").unwrap();

    let mut infer_param = InferParams::new().max_new_tokens(cli.max_tokens);

    let start = Instant::now();

    eprintln!("[rknn3-cli] Running inference...");
    session.run(&mut [llm_input], &mut infer_param)?;

    let total_time = start.elapsed();
    eprintln!();

    if let Ok(state) = session.query_state() {
        eprintln!(
            "[rknn3-cli] Decode: {} tokens, Prefill: {} tokens, Time: {:.2}s",
            state.n_decode_tokens,
            state.n_prefill_tokens,
            total_time.as_secs_f64()
        );
        if state.n_decode_tokens > 0 {
            let tps = state.n_decode_tokens as f64 / total_time.as_secs_f64();
            eprintln!("[rknn3-cli] Speed: {tps:.2} tokens/s");
        }
    } else {
        eprintln!("[rknn3-cli] Time: {:.2}s", total_time.as_secs_f64());
    }

    Ok(())
}
