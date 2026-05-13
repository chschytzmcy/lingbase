//! Configuration management using config-rs.

use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackendConfig {
    pub supported: Vec<String>,
    pub auto_detect: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
    pub test_model_path: String,
    pub context_size: usize,
    pub max_prompt_tokens: usize,
    pub max_generation_tokens: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub backend: BackendConfig,
    pub model: ModelConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config/environment"))
            .build()?;

        config.try_deserialize()
    }
}