//! Domain models expressed with Rust's type system.
//!
//! Domain modeling built around algebraic data types.

mod article;
mod attributes;
mod identifiers;

use crate::error::DomainError;
pub use article::{Article, ArticleSummary};
pub use attributes::{Category, ContentKind, Title};
pub use identifiers::{ArticleId, PageKey, Slug};
use serde::{Deserialize, Deserializer, de::Error as DeError};
use std::str::FromStr;

pub(super) fn deserialize_validated_string<'de, D, T>(
    deserializer: D,
) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr<Err = DomainError>,
{
    let value = String::deserialize(deserializer)?;
    T::from_str(&value).map_err(D::Error::custom)
}
