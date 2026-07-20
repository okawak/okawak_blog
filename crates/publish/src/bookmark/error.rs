use thiserror::Error;

pub type Result<T> = std::result::Result<T, BookmarkError>;

#[derive(Debug, Error)]
pub enum BookmarkError {
    #[error("network request failed: {0}")]
    Network(#[from] reqwest::Error),
}
