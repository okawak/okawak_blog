use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfraError {
    #[error("failed to read artifact file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode artifact json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, InfraError>;
