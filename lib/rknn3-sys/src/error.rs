//! Error types for RKNN3 operations.

/// RKNN3 API error code constants (from rknn3_api.h).
pub mod code {
    pub const FAIL: i32 = -1;
    pub const ARGUMENT_INVALID: i32 = -2;
    pub const MODEL_INVALID: i32 = -3;
    pub const CTX_INVALID: i32 = -4;
    pub const RUN_TASK_FAILED: i32 = -5;
    pub const OUT_OF_MEMORY: i32 = -6;
    pub const TIMEOUT: i32 = -7;
    pub const INPUT_INVALID: i32 = -8;
    pub const OUTPUT_INVALID: i32 = -9;
    pub const DEVICE_UNAVAILABLE: i32 = -10;
    pub const DEVICE_UNMATCH: i32 = -11;
    pub const TARGET_PLATFORM_UNMATCH: i32 = -12;
    pub const COMMUNICATION: i32 = -13;
    pub const MEM_SYNC_FAILED: i32 = -14;

    #[cfg(test)]
    pub const SUCCESS: i32 = 0;

    #[cfg(test)]
    pub const WARN_NPU_CORE_UNUSED: i32 = -100;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // --- RKNN3 API errors (prefixed with `Api`) ---
    #[error("{function}: execution failed (RKNN3_ERR_FAIL)")]
    ApiFail { function: &'static str },
    #[error("{function}: invalid argument (RKNN3_ERR_ARGUMENT_INVALID)")]
    ApiArgumentInvalid { function: &'static str },
    #[error("{function}: invalid model (RKNN3_ERR_MODEL_INVALID)")]
    ApiModelInvalid { function: &'static str },
    #[error("{function}: invalid context (RKNN3_ERR_CTX_INVALID)")]
    ApiCtxInvalid { function: &'static str },
    #[error("{function}: task run failed (RKNN3_ERR_RUN_TASK_FAILED)")]
    ApiRunTaskFailed { function: &'static str },
    #[error("{function}: out of memory (RKNN3_ERR_OUT_OF_MEMORY)")]
    ApiOutOfMemory { function: &'static str },
    #[error("{function}: execution timed out (RKNN3_ERR_TIMEOUT)")]
    ApiTimeout { function: &'static str },
    #[error("{function}: invalid input (RKNN3_ERR_INPUT_INVALID)")]
    ApiInputInvalid { function: &'static str },
    #[error("{function}: invalid output (RKNN3_ERR_OUTPUT_INVALID)")]
    ApiOutputInvalid { function: &'static str },
    #[error("{function}: NPU device unavailable (RKNN3_ERR_DEVICE_UNAVAILABLE)")]
    ApiDeviceUnavailable { function: &'static str },
    #[error("{function}: device mismatch (RKNN3_ERR_DEVICE_UNMATCH)")]
    ApiDeviceUnmatch { function: &'static str },
    #[error("{function}: target platform mismatch (RKNN3_ERR_TARGET_PLATFORM_UNMATCH)")]
    ApiTargetPlatformUnmatch { function: &'static str },
    #[error("{function}: communication error (RKNN3_ERR_COMMUNICATION)")]
    ApiCommunication { function: &'static str },
    #[error("{function}: memory sync failed (RKNN3_ERR_MEM_SYNC_FAILED)")]
    ApiMemSyncFailed { function: &'static str },
    #[error("{function}: unknown RKNN3 error (code: {code})")]
    ApiUnknown { function: &'static str, code: i32 },
    // --- Local errors ---
    #[error("{context} contains a null byte")]
    NulByte { context: &'static str },
    #[error("context handle is null: {context}")]
    NullHandle { context: &'static str },
    #[error("invalid config: {field} — {reason}")]
    InvalidConfig { field: &'static str, reason: String },
}

impl Error {
    pub fn nul_byte(context: &'static str) -> Self {
        Self::NulByte { context }
    }

    pub fn null_handle(context: &'static str) -> Self {
        Self::NullHandle { context }
    }

    pub fn invalid_config(field: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidConfig {
            field,
            reason: reason.into(),
        }
    }

    pub fn check(function: &'static str, code: i32) -> Result<(), Self> {
        if code == 0 {
            Ok(())
        } else {
            Err(Self::from_code(function, code))
        }
    }

    fn from_code(function: &'static str, code: i32) -> Self {
        match code {
            code::FAIL => Self::ApiFail { function },
            code::ARGUMENT_INVALID => Self::ApiArgumentInvalid { function },
            code::MODEL_INVALID => Self::ApiModelInvalid { function },
            code::CTX_INVALID => Self::ApiCtxInvalid { function },
            code::RUN_TASK_FAILED => Self::ApiRunTaskFailed { function },
            code::OUT_OF_MEMORY => Self::ApiOutOfMemory { function },
            code::TIMEOUT => Self::ApiTimeout { function },
            code::INPUT_INVALID => Self::ApiInputInvalid { function },
            code::OUTPUT_INVALID => Self::ApiOutputInvalid { function },
            code::DEVICE_UNAVAILABLE => Self::ApiDeviceUnavailable { function },
            code::DEVICE_UNMATCH => Self::ApiDeviceUnmatch { function },
            code::TARGET_PLATFORM_UNMATCH => Self::ApiTargetPlatformUnmatch { function },
            code::COMMUNICATION => Self::ApiCommunication { function },
            code::MEM_SYNC_FAILED => Self::ApiMemSyncFailed { function },
            _ => Self::ApiUnknown { function, code },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_success() {
        assert!(Error::check("rknn3_init", 0).is_ok());
    }

    #[test]
    fn check_fail() {
        let err = Error::check("rknn3_init", code::FAIL).unwrap_err();
        assert!(matches!(err, Error::ApiFail { .. }));
        assert!(err.to_string().contains("rknn3_init"));
    }

    #[test]
    fn check_timeout() {
        let err = Error::check("rknn3_run", code::TIMEOUT).unwrap_err();
        assert!(matches!(err, Error::ApiTimeout { .. }));
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn check_unknown_code() {
        let err = Error::check("rknn3_query", 99).unwrap_err();
        assert!(matches!(err, Error::ApiUnknown { code: 99, .. }));
    }

    #[test]
    fn check_warning_code() {
        let err = Error::check("rknn3_model_init", code::WARN_NPU_CORE_UNUSED).unwrap_err();
        assert!(matches!(err, Error::ApiUnknown { code: -100, .. }));
    }

    #[test]
    fn nul_byte() {
        let err = Error::nul_byte("model_path");
        assert!(err.to_string().contains("model_path"));
        assert!(err.to_string().contains("null byte"));
    }

    #[test]
    fn null_handle() {
        let err = Error::null_handle("rknn3_run");
        assert!(err.to_string().contains("rknn3_run"));
    }

    #[test]
    fn invalid_config() {
        let err = Error::invalid_config("max_context_len", "must be positive");
        assert!(err.to_string().contains("max_context_len"));
        assert!(err.to_string().contains("must be positive"));
    }

    #[test]
    fn code_constants() {
        assert_eq!(code::SUCCESS, 0);
        assert_eq!(code::FAIL, -1);
        assert_eq!(code::ARGUMENT_INVALID, -2);
        assert_eq!(code::MODEL_INVALID, -3);
        assert_eq!(code::CTX_INVALID, -4);
        assert_eq!(code::RUN_TASK_FAILED, -5);
        assert_eq!(code::OUT_OF_MEMORY, -6);
        assert_eq!(code::TIMEOUT, -7);
        assert_eq!(code::INPUT_INVALID, -8);
        assert_eq!(code::OUTPUT_INVALID, -9);
        assert_eq!(code::DEVICE_UNAVAILABLE, -10);
        assert_eq!(code::DEVICE_UNMATCH, -11);
        assert_eq!(code::TARGET_PLATFORM_UNMATCH, -12);
        assert_eq!(code::COMMUNICATION, -13);
        assert_eq!(code::MEM_SYNC_FAILED, -14);
        assert_eq!(code::WARN_NPU_CORE_UNUSED, -100);
    }
}
