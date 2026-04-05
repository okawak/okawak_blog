use thiserror::Error;

pub type Result<T> = std::result::Result<T, ObsidianError>;

#[derive(Error, Debug)]
pub enum ObsidianError {
    #[error("File system operation failed")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse YAML frontmatter")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Publisher artifact operation failed")]
    PublisherArtifacts(#[from] publisher_artifacts::PublisherArtifactsError),

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

    #[error("Failed to traverse directory")]
    WalkDir(#[from] ignore::Error),

    #[error("Network request failed: {0}")]
    Network(String),

    #[error("Domain validation failed: {0}")]
    Domain(#[from] domain::DomainError),
}
