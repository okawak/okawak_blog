use thiserror::Error;

pub type Result<T> = std::result::Result<T, PublisherArtifactsError>;

#[derive(Debug, Error)]
pub enum PublisherArtifactsError {
    #[error("failed to access local artifact filesystem")]
    Io(#[from] std::io::Error),

    #[error("failed to serialize artifact JSON")]
    Json(#[from] serde_json::Error),
}
