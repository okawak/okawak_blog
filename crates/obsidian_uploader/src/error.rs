use thiserror::Error;

pub type Result<T> = std::result::Result<T, ObsidianError>;

#[derive(Error, Debug)]
pub enum ObsidianError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),
    
    #[error("Path error: {0}")]
    PathError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Parsing error: {0}")]
    ParseError(String),
    
    #[error("Environment variable error: {0}")]
    EnvError(#[from] std::env::VarError),
    
    #[error("Directory walking error: {0}")]
    WalkDirError(#[from] walkdir::Error),
}