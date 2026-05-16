//! API type definitions (OpenAI compatible).

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

fn default_temperature() -> f32 { 0.7 }
fn default_top_p() -> f32 { 0.9 }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: usize,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

// Streaming types
#[derive(Debug, Serialize)]
pub struct StreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Serialize)]
pub struct StreamChoice {
    pub index: usize,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

// Models list types
#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<ModelInfo>,
}

#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

// Metrics types
#[derive(Debug, Serialize)]
pub struct StreamMetrics {
    pub throughput_tokens_per_sec: f64,
    pub time_to_first_token_ms: u64,
    pub end_to_end_latency_ms: u64,
    pub completion_tokens: usize,
    pub inter_token_latency_ms: f64,
    pub p90_latency_ms: u64,
    pub p99_latency_ms: u64,
}

impl StreamMetrics {
    pub fn compute(
        completion_tokens: usize,
        first_token_ms: Option<u64>,
        total_ms: u64,
        inter_token_latency_ms: f64,
        p90_ms: u64,
        p99_ms: u64,
    ) -> Self {
        let throughput = if total_ms > 0 {
            (completion_tokens as f64) / (total_ms as f64 / 1000.0)
        } else {
            0.0
        };
        Self {
            throughput_tokens_per_sec: throughput,
            time_to_first_token_ms: first_token_ms.unwrap_or(0),
            end_to_end_latency_ms: total_ms,
            completion_tokens,
            inter_token_latency_ms,
            p90_latency_ms: p90_ms,
            p99_latency_ms: p99_ms,
        }
    }
}

// Stream metrics state for tracking during streaming
#[derive(Debug, Default)]
pub struct StreamMetricsState {
    pub ttft_ms: u64,
    pub completion_tokens: usize,
    pub inter_token_latencies: Vec<u64>,
    pub last_token_time_ms: u64,
}

impl StreamMetricsState {
    pub fn record_token(&mut self, token_time_ms: u64, is_first: bool) {
        if is_first {
            self.ttft_ms = token_time_ms;
        } else if self.last_token_time_ms > 0 {
            self.inter_token_latencies.push(token_time_ms - self.last_token_time_ms);
        }
        self.last_token_time_ms = token_time_ms;
        self.completion_tokens += 1;
    }

    pub fn compute_itl(&self) -> f64 {
        if self.inter_token_latencies.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.inter_token_latencies.iter().sum();
        sum as f64 / self.inter_token_latencies.len() as f64
    }

    pub fn compute_percentile(latencies: &[u64], p: f64) -> u64 {
        if latencies.is_empty() {
            return 0;
        }
        let mut sorted = latencies.to_vec();
        sorted.sort();
        let idx = ((sorted.len() as f64 * p).floor() as usize).min(sorted.len() - 1);
        sorted[idx]
    }

    pub fn compute_p90(&self) -> u64 {
        Self::compute_percentile(&self.inter_token_latencies, 0.90)
    }

    pub fn compute_p99(&self) -> u64 {
        Self::compute_percentile(&self.inter_token_latencies, 0.99)
    }
}