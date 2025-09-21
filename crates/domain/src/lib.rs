//! Blog Domain - 純粋ドメインロジックとビジネスルール
//!
//! I/Oなし、同期のみの純粋なドメインモデリング。
//! Rustの型システムを活用したビジネスルールの実装。

// ドメインエンティティとバリューオブジェクト
pub mod entities;

// ビジネスルール（純粋関数のみ）
pub mod business_rules;

// ドメインエラー型
pub mod error;

// 再エクスポート
pub use business_rules::*;
pub use entities::*;
pub use error::{DomainError, Result};
