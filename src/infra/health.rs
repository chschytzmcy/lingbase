//! Health check implementation.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use axum::{extract::State, Json, Router, routing::get};
use tower_http::trace::TraceLayer;
use crate::backend::InferenceBackend;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub backend: String,
    pub model_loaded: bool,
    pub slots_available: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_memory_used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_memory_total_bytes: Option<u64>,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub checks: Vec<HealthCheckResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

pub struct HealthCheck {
    backend: Arc<dyn InferenceBackend>,
    start_time: std::time::Instant,
}

impl HealthCheck {
    pub fn new(backend: Arc<dyn InferenceBackend>) -> Self {
        Self {
            backend,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn health(&self) -> HealthResponse {
        let backend_name = self.backend.name().to_string();
        let model_loaded = self.backend.health_check();

        HealthResponse {
            status: if model_loaded { "healthy" } else { "unhealthy" }.to_string(),
            backend: backend_name,
            model_loaded,
            slots_available: 1,
            gpu_memory_used_bytes: None,
            gpu_memory_total_bytes: None,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    pub fn readiness(&self) -> ReadinessResponse {
        let mut checks = vec![];

        let model_loaded = self.backend.health_check();
        checks.push(HealthCheckResult {
            name: "model_loaded".to_string(),
            status: if model_loaded { "ok" } else { "error" }.to_string(),
            detail: if model_loaded { None } else { Some("Model not loaded".to_string()) },
        });

        let backend_ok = self.backend.health_check();
        checks.push(HealthCheckResult {
            name: "backend_responsive".to_string(),
            status: if backend_ok { "ok" } else { "error" }.to_string(),
            detail: None,
        });

        let memory = self.backend.memory_stats();
        let memory_ok = memory.total_bytes == 0 || memory.used_bytes < memory.total_bytes;
        checks.push(HealthCheckResult {
            name: "memory_healthy".to_string(),
            status: if memory_ok { "ok" } else { "warning" }.to_string(),
            detail: Some(format!("{} / {} bytes used", memory.used_bytes, memory.total_bytes)),
        });

        let ready = checks.iter().all(|c| c.status == "ok");

        ReadinessResponse { ready, checks }
    }
}

pub async fn health_handler(State(health): State<Arc<HealthCheck>>) -> Json<HealthResponse> {
    Json(health.health())
}

pub async fn readiness_handler(State(health): State<Arc<HealthCheck>>) -> Json<ReadinessResponse> {
    Json(health.readiness())
}

pub fn create_health_router(health: Arc<HealthCheck>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/health/ready", get(readiness_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(health)
}