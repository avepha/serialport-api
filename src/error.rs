use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerialportApiError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("invalid UTF-8 serial line")]
    InvalidUtf8,

    #[error("serial port error: {0}")]
    SerialPort(#[from] serialport::Error),

    #[error("serial I/O error: {0}")]
    SerialIo(#[from] std::io::Error),

    #[error("connection not found: {0}")]
    ConnectionNotFound(String),

    #[error("command payload must be a JSON object")]
    InvalidCommandPayload,

    #[error("command timed out")]
    CommandTimeout,
}

pub type Result<T> = std::result::Result<T, SerialportApiError>;
