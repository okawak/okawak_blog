//! Blog Core - ドメインロジックとビジネスルール
//!
//! Rustの型システムを活用したドメインモデリングと
//! ビジネスロジックの実装を提供します。

// ドメインモデル（Rustの型システムでモデリング）
pub mod models;

// ビジネスロジック（純粋関数 + 構造体メソッド）
pub mod services;

// ユースケース（async関数群）
pub mod use_cases;

// エラー型（thiserror + Result型）
pub mod error;

// 外部サービス用トレイト（dependency inversion）
pub mod ports;

// 再エクスポート
pub use error::{CoreError, Result};
pub use models::*;
pub use services::*;
pub use use_cases::*;
