use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ModelNotFound,
    ModelNotInstalled,
    CapabilityUnsupported,
    RunnerNotFound,
    RunnerNotInstalled,
    RunnerUnsupportedOnPlatform,
    InferenceNotImplemented,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::ModelNotFound => "model_not_found",
            ErrorCode::ModelNotInstalled => "model_not_installed",
            ErrorCode::CapabilityUnsupported => "capability_unsupported",
            ErrorCode::RunnerNotFound => "runner_not_found",
            ErrorCode::RunnerNotInstalled => "runner_not_installed",
            ErrorCode::RunnerUnsupportedOnPlatform => "runner_unsupported_on_platform",
            ErrorCode::InferenceNotImplemented => "inference_not_implemented",
        }
    }
}

#[derive(Debug, Error)]
pub enum TakokitError {
    #[error("{feature} is not implemented yet: {reason}")]
    NotImplemented {
        feature: &'static str,
        reason: &'static str,
    },

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("model error: {0}")]
    Model(String),

    #[error("{message}")]
    Resolution { code: ErrorCode, message: String },

    #[error("audio error: {0}")]
    Audio(String),
}

pub type TakokitResult<T> = Result<T, TakokitError>;
