//! Blog Domain - 純粋ドメインロジックとビジネスルール
//!
//! I/Oなし、同期のみの純粋なドメインモデリング。
//! Rustの型システムを活用したビジネスルールの実装。

// ドメインエンティティとバリューオブジェクト
pub mod artifact_document;
pub mod entities;
pub mod publishable;
pub mod site_page;

// ビジネスルール（純粋関数のみ）
pub mod business_rules;

// ドメインエラー型
pub mod error;

// 再エクスポート
pub use artifact_document::*;
pub use business_rules::*;
pub use entities::*;
pub use error::{DomainError, Result};
pub use publishable::*;
pub use site_page::*;
