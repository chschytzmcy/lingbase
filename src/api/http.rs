//! HTTP handlers using Axum.

use axum::{Router, routing::post, Json, extract::State, http::StatusCode};
use std::sync::Arc;
use crate::api::types::{ChatCompletionRequest, ChatCompletionResponse, Message, Choice, Usage};
use crate::backend::{InferenceBackend, InferenceConfig};
use crate::infra::health::HealthCheck;
use crate::infra::logging::RequestLogger;
use uuid::Uuid;

pub struct AppState {
    pub backend: Arc<dyn InferenceBackend>,
    pub health: Arc<HealthCheck>,
}

pub async fn chat_completions_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, (StatusCode, String)> {
    let request_id = Uuid::new_v4();
    let logger = RequestLogger::new(request_id);

    logger.log_request_start(&request.model, request.messages.iter().map(|m| m.content.len()).sum());

    let _prompt = request.messages.iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let start = std::time::Instant::now();
    let inf_config = InferenceConfig {
        max_tokens: request.max_tokens.unwrap_or(256),
        temperature: request.temperature,
        top_p: request.top_p,
        top_k: 40,
        repeat_penalty: 1.1,
        timeout_ms: request.timeout_ms,
    };

    let result = state.backend.forward(&[], &inf_config)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let inference_ms = start.elapsed().as_millis() as u64;
    logger.log_request_complete(result.tokens.len(), inference_ms, result.first_token_ms);

    let response = ChatCompletionResponse {
        id: format!("chatcmpl-{}", request_id),
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: request.model,
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: "assistant".to_string(),
                content: "Response placeholder".to_string(),
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens: 0,
            completion_tokens: result.tokens.len(),
            total_tokens: 0,
        },
    };

    Ok(Json(response))
}

pub fn create_app_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions_handler))
        .with_state(state)
}