use crate::error::{DomainError, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt, str::FromStr};

/// Ordered category-relative directory segments used for article grouping.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SectionPath(Vec<String>);

impl SectionPath {
    pub fn new(segments: Vec<String>) -> Result<Self> {
        if segments.iter().any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || s.contains(['/', '\\'])
                || s.chars().any(char::is_control)
        }) {
            return Err(DomainError::validation("invalid public section"));
        }
        Ok(Self(segments))
    }

    pub fn segments(&self) -> &[String] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'de> Deserialize<'de> for SectionPath {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(Vec::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Article title with business-rule validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Title(String);

impl Title {
    pub fn new(value: String) -> Result<Self> {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::InvalidTitle {
                reason: "cannot be empty".to_string(),
            });
        }

        if trimmed.chars().count() > 200 {
            return Err(DomainError::InvalidTitle {
                reason: "must not exceed 200 characters".to_string(),
            });
        }

        // Preserve source text: normalization would change translation fingerprints.
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Normalize surrounding whitespace explicitly at the presentation boundary.
    pub fn trimmed(self) -> Self {
        Self(self.0.trim().to_owned())
    }
}

impl FromStr for Title {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s.to_string())
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for Title {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        super::deserialize_validated_string(deserializer)
    }
}

/// RFC 3339 timestamp used by publishable content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Timestamp(String);

impl Timestamp {
    pub fn new(value: String) -> Result<Self> {
        chrono::DateTime::parse_from_rfc3339(value.trim()).map_err(|_| {
            DomainError::InvalidTimestamp {
                value: value.to_string(),
            }
        })?;

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Normalize surrounding whitespace explicitly at the presentation boundary.
    pub fn trimmed(self) -> Self {
        Self(self.0.trim().to_owned())
    }
}

impl FromStr for Timestamp {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s.to_string())
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        super::deserialize_validated_string(deserializer)
    }
}

/// Category constrained by an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Category {
    Tech,
    Daily,
    Statistics,
    Physics,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Tech => "tech",
            Category::Daily => "daily",
            Category::Statistics => "statistics",
            Category::Physics => "physics",
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Category {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "tech" => Ok(Category::Tech),
            "daily" => Ok(Category::Daily),
            "statistics" => Ok(Category::Statistics),
            "physics" => Ok(Category::Physics),
            _ => Err(DomainError::InvalidCategory {
                category: s.to_string(),
            }),
        }
    }
}

impl<'de> Deserialize<'de> for Category {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        super::deserialize_validated_string(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::{Category, SectionPath, Timestamp, Title};

    #[test]
    fn test_section_path_exposes_ordered_segments() {
        let path = SectionPath::new(vec!["rust".to_string(), "async".to_string()]).unwrap();

        assert_eq!(path.segments(), ["rust", "async"]);
        assert!(!path.is_empty());
        assert!(SectionPath::default().is_empty());
    }

    #[test]
    fn test_section_path_serialization_remains_an_array() {
        let path = SectionPath::new(vec!["rust".to_string(), "async".to_string()]).unwrap();

        let json = serde_json::to_string(&path).unwrap();
        let deserialized: SectionPath = serde_json::from_str(&json).unwrap();

        assert_eq!(json, r#"["rust","async"]"#);
        assert_eq!(deserialized, path);
    }

    #[test]
    fn test_section_path_rejects_invalid_segments_when_created_or_deserialized() {
        for segment in [
            "",
            ".",
            "..",
            "rust/async",
            "rust\\async",
            "line\nbreak",
            "\0",
        ] {
            assert!(SectionPath::new(vec![segment.into()]).is_err());
            let json = serde_json::json!([segment]);
            assert!(serde_json::from_value::<SectionPath>(json).is_err());
        }
        assert!(SectionPath::new(Vec::new()).unwrap().is_empty());
    }

    #[test]
    fn test_title_deserializes_without_rewriting_valid_text() {
        let title: Title = serde_json::from_str(r#""  Intro  ""#).unwrap();
        assert_eq!(title.as_str(), "  Intro  ");
    }

    #[test]
    fn test_title_validates_unicode_character_count() {
        assert!(Title::new("あ".repeat(200)).is_ok());
        assert!(Title::new("あ".repeat(201)).is_err());
    }

    #[test]
    fn test_timestamp_accepts_rfc3339_without_rewriting_valid_text() {
        let timestamp = Timestamp::new("  2025-01-01T00:00:00+09:00  ".to_string()).unwrap();

        assert_eq!(timestamp.as_str(), "  2025-01-01T00:00:00+09:00  ");
    }

    #[test]
    fn test_timestamp_rejects_invalid_values() {
        for value in ["", "   ", "2025-01-01", "not-a-timestamp"] {
            assert!(Timestamp::new(value.to_string()).is_err());
        }
    }

    #[test]
    fn test_category_deserializes_case_insensitively() {
        let category: Category = serde_json::from_str(r#""TECH""#).unwrap();
        assert_eq!(category, Category::Tech);
    }
}
