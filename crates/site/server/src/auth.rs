//! Authentication & Authorization - 認証・認可

pub mod extractors;

pub use extractors::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("認証が必要です")]
    AuthenticationRequired,

    #[error("認証情報が無効です")]
    InvalidCredentials,

    #[error("アクセス権限がありません")]
    InsufficientPermissions,

    #[error("トークンが無効です: {reason}")]
    InvalidToken { reason: String },
}
