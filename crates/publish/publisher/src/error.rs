use thiserror::Error;

pub type Result<T> = std::result::Result<T, ObsidianError>;

#[derive(Error, Debug)]
pub enum ObsidianError {
    #[error("File system operation failed")]
    Io(#[from] std::io::Error),

    #[error("Obsidian publisher operation failed: {0}")]
    Ingest(#[from] ingest::IngestError),

    #[error("Publisher artifact operation failed")]
    Artifacts(#[from] artifacts::ArtifactsError),

    #[error("Blocking task failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("Invalid file path: {0}")]
    Path(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Failed to parse file content: {0}")]
    Parse(String),

    #[error("Environment variable not found or invalid")]
    Env(#[from] std::env::VarError),

    #[error("Domain validation failed: {0}")]
    Domain(#[from] domain::DomainError),
}
