use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerialportApiError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("invalid UTF-8 serial line")]
    InvalidUtf8,

    #[error("command timed out")]
    CommandTimeout,
}

pub type Result<T> = std::result::Result<T, SerialportApiError>;
