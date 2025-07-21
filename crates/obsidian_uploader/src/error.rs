use thiserror::Error;

pub type Result<T> = std::result::Result<T, ObsidianError>;

#[derive(Error, Debug)]
pub enum ObsidianError {
    #[error("File system operation failed")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse YAML frontmatter")]
    YamlError(#[from] serde_yaml::Error),

    #[error("Invalid file path: {0}")]
    PathError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Failed to parse file content: {0}")]
    ParseError(String),

    #[error("Environment variable not found or invalid")]
    EnvError(#[from] std::env::VarError),

    #[error("Failed to traverse directory")]
    WalkDirError(#[from] ignore::Error),

    #[error("Network request failed: {0}")]
    NetworkError(String),
}
