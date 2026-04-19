//! Domain models expressed with Rust's type system.
//!
//! Domain modeling built around algebraic data types.

mod article;
mod attributes;
mod identifiers;

pub use article::{Article, ArticleSummary};
pub use attributes::{Category, ContentKind, Title};
pub use identifiers::{ArticleId, PageKey, Slug};
