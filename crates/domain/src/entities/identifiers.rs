use crate::error::{DomainError, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt, str::FromStr};

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
                super::deserialize_validated_string(deserializer)
            }
        }
    };
}

/// URL-safe slug identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Slug(String);

impl Slug {
    pub fn new(value: String) -> Result<Self> {
        if value.is_empty() {
            return Err(DomainError::InvalidSlug {
                slug: "cannot be empty".to_string(),
            });
        }

        if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(DomainError::InvalidSlug {
                slug: "must contain only ASCII alphanumeric characters, hyphens, and underscores"
                    .to_string(),
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
                path: "page key cannot be empty".to_string(),
            });
        }

        if value == "home" {
            return Err(DomainError::InvalidPath {
                path: "home is reserved for the home page".to_string(),
            });
        }

        if !value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return Err(DomainError::InvalidPath {
                path: "page key may contain only lowercase ASCII letters, digits, hyphens, and underscores"
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
    use super::{PageKey, Slug};

    #[test]
    fn test_page_key_rejects_reserved_home_key() {
        let error = PageKey::new("home".to_string()).unwrap_err();

        assert!(error.to_string().contains("reserved"));
    }

    #[test]
    fn test_page_key_rejects_invalid_characters() {
        for value in ["about/team", "About"] {
            assert!(PageKey::new(value.to_string()).is_err());
        }
    }

    #[test]
    fn test_slug_deserializes_with_validation() {
        let slug: Slug = serde_json::from_str(r#""intro00000001""#).unwrap();
        assert_eq!(slug.as_str(), "intro00000001");
    }

    #[test]
    fn test_slug_deserialization_rejects_invalid_value() {
        let error = serde_json::from_str::<Slug>(r#""bad slug""#).unwrap_err();
        assert!(error.to_string().contains("slug"));
    }
}
