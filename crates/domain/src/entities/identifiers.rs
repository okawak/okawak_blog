//! Type-safe identifier types for domain entities.

use crate::error::{DomainError, Result};
use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use std::{fmt, str::FromStr};

fn deserialize_validated_string<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr<Err = DomainError>,
{
    let value = String::deserialize(deserializer)?;
    T::from_str(&value).map_err(D::Error::custom)
}

macro_rules! impl_display_and_deserialize {
    ($type:ty) => {
        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserialize_validated_string(deserializer)
            }
        }
    };
}

/// Type-safe article identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ArticleId(String);

impl ArticleId {
    pub fn new() -> Self {
        // Simple ID generation for now. A production system would use UUIDs externally.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .hash(&mut hasher);
        let id = format!("article_{}", hasher.finish());
        Self(id)
    }

    pub fn from_string(id: String) -> Result<Self> {
        if id.is_empty() {
            return Err(DomainError::InvalidId {
                id: "IDは空にできません".to_string(),
            });
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ArticleId {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for ArticleId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_string(s.to_string())
    }
}

impl_display_and_deserialize!(ArticleId);
/// URL-safe slug identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Slug(String);

impl Slug {
    pub fn new(value: String) -> Result<Self> {
        if value.is_empty() {
            return Err(DomainError::InvalidSlug {
                slug: "スラッグは空にできません".to_string(),
            });
        }

        if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(DomainError::InvalidSlug {
                slug: "スラッグは英数字、ハイフン、アンダースコアのみ使用可能です".to_string(),
            });
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Slug {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s.to_string())
    }
}

impl_display_and_deserialize!(Slug);

/// Single path-segment page key used for generated static pages.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct PageKey(String);

impl PageKey {
    pub fn new(value: String) -> Result<Self> {
        if value.is_empty() {
            return Err(DomainError::InvalidPath {
                path: "ページキーは空にできません".to_string(),
            });
        }

        if !value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return Err(DomainError::InvalidPath {
                path: "ページキーは英小文字、数字、ハイフン、アンダースコアのみ使用可能です"
                    .to_string(),
            });
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for PageKey {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s.to_string())
    }
}

impl_display_and_deserialize!(PageKey);

#[cfg(test)]
mod tests {
    use super::Slug;

    #[test]
    fn test_slug_deserializes_with_validation() {
        let slug: Slug = serde_json::from_str(r#""intro00000001""#).unwrap();
        assert_eq!(slug.as_str(), "intro00000001");
    }

    #[test]
    fn test_slug_deserialization_rejects_invalid_value() {
        let error = serde_json::from_str::<Slug>(r#""bad slug""#).unwrap_err();
        assert!(error.to_string().contains("スラッグ"));
    }
}
