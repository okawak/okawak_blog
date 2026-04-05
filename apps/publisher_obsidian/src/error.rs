use thiserror::Error;

pub type Result<T> = std::result::Result<T, PublisherObsidianError>;

#[derive(Debug, Error)]
pub enum PublisherObsidianError {
    #[error("file system operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse YAML frontmatter: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("failed to parse file content: {0}")]
    Parse(String),

    #[error("failed to traverse directory: {0}")]
    WalkDir(#[from] ignore::Error),
}
