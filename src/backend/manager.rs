//! BackendManager - 多后端生命周期管理
//!
//! 管理多个后端的加载、卸载、健康检查等生命周期。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::infra::config::ModelConfig;
use crate::backend::traits::LLMBackend;
use crate::backend::error::BackendError;
use crate::backend::types::ResourceType;

/// BackendManager - 多后端管理器
///
/// 管理多个后端的生命周期，支持多模型热加载。
pub struct BackendManager {
    backends: RwLock<HashMap<String, Arc<dyn LLMBackend>>>,
    #[allow(dead_code)]
    resource_capacity: Vec<ResourceType>,
}

impl BackendManager {
    pub fn new() -> Self {
        Self {
            backends: RwLock::new(HashMap::new()),
            resource_capacity: Vec::new(),
        }
    }

    /// 加载后端
    pub async fn load(&self, config: &ModelConfig) -> Result<(), BackendError> {
        let backend_type = config.backend_type()
            .ok_or_else(|| BackendError::UnknownBackend("No backend type configured".to_string()))?;

        info!(backend_type = backend_type, model = %config.name, "Loading backend");

        match backend_type {
            "llama-cpu" | "cpu" => {
                use crate::backend::cpu::CpuBackend;
                let backend = CpuBackend::new(&config.model_path, config.context_size as i32)
                    .map_err(|e| BackendError::InferenceFailed(e.to_string()))?;
                let mut backends = self.backends.write().await;
                backends.insert(config.name.clone(), Arc::new(backend));
                info!(backend_type = backend_type, model = %config.name, "Backend loaded");
                return Ok(());
            }
            "rknn3" => {
                #[cfg(feature = "rknn3")]
                {
                    use crate::backend::runner_rknn3::Rknn3Backend;
                    let backend = Rknn3Backend::init(config).await?;
                    let mut backends = self.backends.write().await;
                    backends.insert(config.name.clone(), Arc::new(backend));
                    info!(backend_type = backend_type, model = %config.name, "Backend loaded");
                    return Ok(());
                }
                #[cfg(not(feature = "rknn3"))]
                {
                    return Err(BackendError::BackendNotAvailable(
                        "RKNN3 not available (compile with --features rknn3)".to_string()
                    ));
                }
            }
            _ => {
                return Err(BackendError::UnknownBackend(backend_type.to_string()));
            }
        }
    }

    /// 获取后端
    pub async fn get(&self, name: &str) -> Option<Arc<dyn LLMBackend>> {
        let backends = self.backends.read().await;
        backends.get(name).cloned()
    }

    /// 卸载后端
    pub async fn unload(&self, name: &str) -> Result<(), BackendError> {
        let mut backends = self.backends.write().await;
        if backends.remove(name).is_some() {
            info!(model = name, "Backend unloaded");
            Ok(())
        } else {
            warn!(model = name, "Backend not found");
            Err(BackendError::BackendNotInitialized)
        }
    }

    /// 获取所有后端名称
    pub async fn list(&self) -> Vec<String> {
        let backends = self.backends.read().await;
        backends.keys().cloned().collect()
    }

    /// 检查后端健康状态
    pub async fn health_check(&self, name: &str) -> bool {
        if let Some(backend) = self.get(name).await {
            backend.health_check()
        } else {
            false
        }
    }

    /// 清理不活跃的后端
    pub async fn cleanup_inactive(&self, _timeout_secs: u64) -> Result<(), BackendError> {
        // TODO: 实现基于最后活跃时间的清理逻辑
        Ok(())
    }

    /// 获取所有后端的资源使用情况
    pub async fn resource_usage(&self) -> Vec<(String, Vec<ResourceType>)> {
        let backends = self.backends.read().await;
        backends
            .iter()
            .map(|(name, backend)| (name.clone(), backend.resource_usage()))
            .collect()
    }
}

impl Default for BackendManager {
    fn default() -> Self {
        Self::new()
    }
}