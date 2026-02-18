use thiserror::Error;

#[derive(Debug, Error)]
pub enum PeriphError {
    #[error("unsupported peripheral kind: {0}")]
    UnsupportedKind(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serial error: {0}")]
    Serial(#[from] serialport::Error),
    #[error("hid error: {0}")]
    Hid(#[from] hidapi::HidError),
}

pub type Result<T> = std::result::Result<T, PeriphError>;
