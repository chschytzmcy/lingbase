//! Lingbase - Edge LLM Inference Service

use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use lingbase::{
    backend::BackendFactory,
    infra::{AppConfig, HealthCheck, init_logging},
    api::{create_app_router, AppState},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging()?;

    info!("Starting Lingbase Edge LLM Inference Service");

    let config = AppConfig::load()
        .expect("Failed to load configuration");

    info!(
        host = %config.server.host,
        port = %config.server.port,
        model_path = %config.model.test_model_path,
        "Configuration loaded"
    );

    let backend = BackendFactory::create(
        BackendFactory::auto_detect(),
        std::path::Path::new(&config.model.test_model_path),
        config.model.context_size as i32,
    ).expect("Failed to create backend");

    info!(backend = backend.name(), "Backend initialized");

    let health = Arc::new(HealthCheck::new(backend.clone()));

    let app_state = Arc::new(AppState {
        backend,
        health: health.clone(),
    });

    let app = create_app_router(app_state)
        .merge(lingbase::infra::health::create_health_router(health));

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;

    info!(address = %addr, "Server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}