//! Error types - 旧アーキテクチャ互換性スタブ
//! 将来的に削除予定

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
