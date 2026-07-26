use thiserror::Error;

pub type Result<T> = std::result::Result<T, PublishError>;

#[derive(Error, Debug)]
pub enum PublishError {
    #[error("file system operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse YAML frontmatter: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("failed to serialize artifact JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("bookmark network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("blocking task failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error("invalid file path: {0}")]
    InvalidPath(String),

    #[error("invalid Obsidian source directory: {0}")]
    InvalidSourceDirectory(String),

    #[error("failed to parse file content: {0}")]
    Parse(String),

    #[error("artifact validation failed: {0}")]
    ArtifactValidation(String),

    #[error("domain validation failed: {0}")]
    Domain(#[from] domain::DomainError),

    #[error("publisher rejected {count} invalid content file(s)")]
    ContentErrors { count: usize },
}
