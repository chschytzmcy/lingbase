//! Configuration management using config-rs.

use std::path::PathBuf;

use config::{Config, ConfigError, File};
use serde::Deserialize;

/// 服务器配置
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
}

fn default_workers() -> usize {
    1
}

/// 后端配置
#[derive(Debug, Deserialize, Clone, Default)]
pub struct BackendConfig {
    /// 后端类型："llama-cpu", "llama-cuda", "rknn3", "proxy"
    pub backend_type: String,
    /// 预编译库目录（可选，用于 llama.cpp）
    pub lib_dir: Option<String>,
}

/// 推理选项配置
#[derive(Debug, Deserialize, Clone)]
pub struct InferenceConfig {
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_repeat_penalty")]
    pub repeat_penalty: f32,
}

fn default_max_tokens() -> usize { 256 }
fn default_temperature() -> f32 { 0.7 }
fn default_top_p() -> f32 { 0.9 }
fn default_top_k() -> usize { 40 }
fn default_repeat_penalty() -> f32 { 1.1 }

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            top_p: default_top_p(),
            top_k: default_top_k(),
            repeat_penalty: default_repeat_penalty(),
        }
    }
}

/// 模型配置
#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
    /// 模型名称
    pub name: String,
    /// 模型文件路径
    pub model_path: PathBuf,
    /// 上下文窗口大小
    pub context_size: usize,
    /// 推理选项
    #[serde(default)]
    pub inference: InferenceConfig,
    /// 后端配置
    #[serde(default)]
    pub backend: BackendConfig,
    /// 启动时自动加载
    #[serde(default)]
    pub auto_load: bool,
}

impl ModelConfig {
    /// 获取后端类型
    pub fn backend_type(&self) -> Option<&str> {
        if self.backend.backend_type.is_empty() {
            None
        } else {
            Some(&self.backend.backend_type)
        }
    }
}

/// 应用配置
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    #[serde(default)]
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