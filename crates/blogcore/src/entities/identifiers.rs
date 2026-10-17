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

/// Original tag identity. Whitespace and case are preserved for catalog lookup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct TagId(String);

impl TagId {
    pub fn new(value: String) -> Result<Self> {
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(DomainError::validation("invalid tag identifier"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for TagId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s.to_owned())
    }
}

impl std::borrow::Borrow<str> for TagId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl_display_and_deserialize!(TagId);

/// SHA-256 fingerprint encoded as 64 hexadecimal characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    pub fn new(value: String) -> Result<Self> {
        if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(DomainError::validation("invalid SHA-256 digest"));
        }
        Ok(Self(value))
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes.iter().map(|b| format!("{b:02x}")).collect())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Sha256Digest {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s.to_owned())
    }
}

impl_display_and_deserialize!(Sha256Digest);

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
    use super::{PageKey, Sha256Digest, Slug, TagId};

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

    #[test]
    fn tag_ids_validate_without_changing_catalog_identity() {
        for value in ["", "   ", "line\nbreak", "tab\tname", "\0"] {
            assert!(TagId::new(value.into()).is_err());
            assert!(serde_json::from_value::<TagId>(serde_json::json!(value)).is_err());
        }
        for value in ["Rust", "日本語", " tag ", "Rust's tips"] {
            let id: TagId = value.parse().unwrap();
            assert_eq!(id.as_str(), value);
            assert_eq!(serde_json::to_value(id).unwrap(), serde_json::json!(value));
        }
    }

    #[test]
    fn sha256_digests_validate_length_and_hex_without_rewriting_input() {
        for value in [
            String::new(),
            "a".repeat(63),
            "a".repeat(65),
            "g".repeat(64),
        ] {
            assert!(Sha256Digest::new(value.clone()).is_err());
            assert!(serde_json::from_value::<Sha256Digest>(serde_json::json!(value)).is_err());
        }
        for value in ["ab".repeat(32), "AB".repeat(32)] {
            let digest: Sha256Digest = value.parse().unwrap();
            assert_eq!(digest.as_str(), value);
            assert_eq!(
                serde_json::to_value(digest).unwrap(),
                serde_json::json!(value)
            );
        }
        assert_eq!(
            Sha256Digest::from_bytes([0xab; 32]).as_str(),
            "ab".repeat(32)
        );
    }
}
