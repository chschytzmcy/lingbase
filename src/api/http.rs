//! HTTP handlers using Axum.

use axum::{Router, routing::{post, get}, Json, extract::State, http::StatusCode, response::{sse::{Event, Sse}, IntoResponse}};
use futures::stream::StreamExt as _;
use std::sync::Arc;
use crate::api::types::{ChatCompletionRequest, ChatCompletionResponse, Message, Choice, Usage, StreamChunk, StreamChoice, Delta, ModelsResponse, ModelInfo};
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
) -> Result<axum::response::Response, (StatusCode, String)> {
    let request_id = Uuid::new_v4();
    let logger = RequestLogger::new(request_id);

    logger.log_request_start(&request.model, request.messages.iter().map(|m| m.content.len()).sum());

    // Build prompt from messages using Qwen3 chat template
    let mut prompt = request.messages.iter()
        .map(|m| format!("<|im_start|>{}\n{}<|im_end|>", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    // Add assistant start token so model knows to generate response
    // Add /no_think to disable Qwen3 thinking mode
    prompt.push_str("<|im_start|>assistant\n/no_think\n");

    let input_tokens = state.backend.tokenize(&prompt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Tokenization failed: {}", e)))?;

    let inf_config = InferenceConfig {
        max_tokens: request.max_tokens.unwrap_or(256),
        temperature: request.temperature,
        top_p: request.top_p,
        top_k: 40,
        repeat_penalty: 1.1,
        timeout_ms: request.timeout_ms,
    };

    if request.stream {
        // Streaming response
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let model = request.model.clone();
        let id = format!("chatcmpl-{}", request_id);

        let stream = state.backend.forward_stream(&input_tokens, &inf_config)
            .map(move |result| {
                let event = match result {
                    Ok(token) => {
                        if token.is_done {
                            // Final chunk with finish_reason
                            let chunk = StreamChunk {
                                id: id.clone(),
                                object: "chat.completion.chunk".to_string(),
                                created,
                                model: model.clone(),
                                choices: vec![StreamChoice {
                                    index: 0,
                                    delta: Delta { role: None, content: None },
                                    finish_reason: Some("stop".to_string()),
                                }],
                            };
                            Event::default().data(serde_json::to_string(&chunk).unwrap_or_default())
                        } else if token.is_first {
                            // First chunk includes role
                            let chunk = StreamChunk {
                                id: id.clone(),
                                object: "chat.completion.chunk".to_string(),
                                created,
                                model: model.clone(),
                                choices: vec![StreamChoice {
                                    index: 0,
                                    delta: Delta {
                                        role: Some("assistant".to_string()),
                                        content: Some(token.text),
                                    },
                                    finish_reason: None,
                                }],
                            };
                            Event::default().data(serde_json::to_string(&chunk).unwrap_or_default())
                        } else {
                            // Regular chunk
                            let chunk = StreamChunk {
                                id: id.clone(),
                                object: "chat.completion.chunk".to_string(),
                                created,
                                model: model.clone(),
                                choices: vec![StreamChoice {
                                    index: 0,
                                    delta: Delta { role: None, content: Some(token.text) },
                                    finish_reason: None,
                                }],
                            };
                            Event::default().data(serde_json::to_string(&chunk).unwrap_or_default())
                        }
                    }
                    Err(e) => {
                        Event::default().data(format!("error: {}", e))
                    }
                };
                Ok::<_, std::convert::Infallible>(event)
            });

        Ok(Sse::new(stream).into_response())
    } else {
        // Non-streaming response (original logic)
        let start = std::time::Instant::now();
        let result = state.backend.forward(&input_tokens, &inf_config)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Inference failed: {}", e)))?;

        let inference_ms = start.elapsed().as_millis() as u64;
        logger.log_request_complete(result.tokens.len(), inference_ms, result.first_token_ms);

        // Detokenize output and trim leading whitespace (from /no_think formatting)
        let response_text = state.backend.detokenize(&result.tokens)
            .unwrap_or_else(|_| "Failed to decode response".to_string())
            .trim_start()
            .to_string();

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
                    content: response_text,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Usage {
                prompt_tokens: input_tokens.len(),
                completion_tokens: result.tokens.len(),
                total_tokens: input_tokens.len() + result.tokens.len(),
            },
        };

        Ok(Json(response).into_response())
    }
}

pub fn create_app_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions_handler))
        .route("/v1/models", get(models_handler))
        .with_state(state)
}

pub async fn models_handler() -> Json<ModelsResponse> {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Json(ModelsResponse {
        object: "list".to_string(),
        data: vec![
            ModelInfo {
                id: "Qwen3-4B".to_string(),
                object: "model".to_string(),
                created,
                owned_by: "Qwen".to_string(),
            },
        ],
    })
}