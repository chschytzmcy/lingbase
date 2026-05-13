//! Structured logging initialization using tracing.

use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_subscriber::fmt::format::FmtSpan;
use std::io;

pub fn init_logging() -> io::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
        .json();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

#[derive(Debug, Clone)]
pub struct RequestLogger {
    pub request_id: uuid::Uuid,
    pub client_id: Option<String>,
}

impl RequestLogger {
    pub fn new(request_id: uuid::Uuid) -> Self {
        Self { request_id, client_id: None }
    }

    pub fn with_client_id(mut self, client_id: String) -> Self {
        self.client_id = Some(client_id);
        self
    }

    pub fn log_request_start(&self, model: &str, prompt_tokens: usize) {
        tracing::info!(
            request_id = %self.request_id,
            client_id = %self.client_id.as_deref().unwrap_or("anonymous"),
            model = %model,
            prompt_tokens = prompt_tokens,
            "Request received"
        );
    }

    pub fn log_request_complete(&self, total_tokens: usize, inference_ms: u64, first_token_ms: Option<u64>) {
        tracing::info!(
            request_id = %self.request_id,
            total_tokens = total_tokens,
            inference_ms = inference_ms,
            first_token_ms = first_token_ms,
            "Request completed"
        );
    }

    pub fn log_error(&self, error_code: &str, detail: &str) {
        tracing::error!(
            request_id = %self.request_id,
            error_code = %error_code,
            error_detail = %detail,
            "Inference error"
        );
    }
}