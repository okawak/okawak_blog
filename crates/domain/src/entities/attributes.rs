//! Domain attribute and classification types.

use crate::error::{DomainError, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt, str::FromStr};


/// Article title with business-rule validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Title(String);

impl Title {
    pub fn new(value: String) -> Result<Self> {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::InvalidTitle {
                reason: "タイトルは空にできません".to_string(),
            });
        }

        if trimmed.len() > 200 {
            return Err(DomainError::InvalidTitle {
                reason: "タイトルは200文字以内である必要があります".to_string(),
            });
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
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

    pub fn display_name(&self) -> &'static str {
        match self {
            Category::Tech => "技術",
            Category::Daily => "日常",
            Category::Statistics => "統計学",
            Category::Physics => "物理学",
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

/// Content role declared in Obsidian frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    #[default]
    Article,
    Category,
    Page,
    Home,
}

impl ContentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentKind::Article => "article",
            ContentKind::Category => "category",
            ContentKind::Page => "page",
            ContentKind::Home => "home",
        }
    }
}

impl fmt::Display for ContentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ContentKind {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "article" => Ok(ContentKind::Article),
            "category" => Ok(ContentKind::Category),
            "page" => Ok(ContentKind::Page),
            "home" => Ok(ContentKind::Home),
            _ => Err(DomainError::validation("kind")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Category, Title};

    #[test]
    fn test_title_deserializes_with_trimmed_value() {
        let title: Title = serde_json::from_str(r#""  Intro  ""#).unwrap();
        assert_eq!(title.as_str(), "Intro");
    }

    #[test]
    fn test_category_deserializes_case_insensitively() {
        let category: Category = serde_json::from_str(r#""TECH""#).unwrap();
        assert_eq!(category, Category::Tech);
    }
}
