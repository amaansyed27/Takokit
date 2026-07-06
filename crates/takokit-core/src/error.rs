use thiserror::Error;

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

    #[error("audio error: {0}")]
    Audio(String),
}

pub type TakokitResult<T> = Result<T, TakokitError>;
