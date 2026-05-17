//! 后端类型定义

/// 后端类型枚举（新版）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// Llama.cpp CPU 后端
    LlamaCpu,
    /// Llama.cpp CUDA 后端
    LlamaCuda,
    /// RKNN3 NPU 后端
    Rknn3,
    /// 远程代理后端
    Proxy,
}

impl Default for BackendType {
    fn default() -> Self {
        BackendType::LlamaCpu
    }
}

impl BackendType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "llama-cpu" | "cpu" => Some(BackendType::LlamaCpu),
            "llama-cuda" | "cuda" => Some(BackendType::LlamaCuda),
            "rknn3" | "rknn" => Some(BackendType::Rknn3),
            "proxy" => Some(BackendType::Proxy),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BackendType::LlamaCpu => "llama-cpu",
            BackendType::LlamaCuda => "llama-cuda",
            BackendType::Rknn3 => "rknn3",
            BackendType::Proxy => "proxy",
        }
    }
}

/// 资源类型
#[derive(Debug, Clone)]
pub struct ResourceType {
    pub name: String,
    pub used: u64,
    pub total: u64,
}

/// 推理完成原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    /// 正常停止（遇到 EOS）
    Stop,
    /// 达到最大 token 数
    Length,
    /// 触发 stop 序列
    StopSequence,
    /// 模型生成结束
    ModelEnd,
    /// 错误
    Error,
}