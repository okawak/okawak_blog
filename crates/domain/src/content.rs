//! Versioned, public Markdown frontmatter shared by export and publish.
//! Serialization formats and filesystem access belong to the callers.
use crate::{Category, DomainError, PageKey, Result, SectionPath, Slug, Timestamp, Title};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    #[default]
    Ja,
    En,
}

impl Locale {
    pub const ALL: [Self; 2] = [Self::Ja, Self::En];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ja => "ja",
            Self::En => "en",
        }
    }

    pub fn path(self, japanese_path: &str) -> String {
        match self {
            Self::Ja => japanese_path.to_owned(),
            Self::En if japanese_path == "/" => "/en".into(),
            Self::En => format!("/en{japanese_path}"),
        }
    }

    pub fn artifact_key(self, key: &str) -> String {
        match self {
            Self::Ja => key.to_owned(),
            Self::En => format!("en/{key}"),
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Locale {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self> {
        match value {
            "ja" => Ok(Self::Ja),
            "en" => Ok(Self::En),
            _ => Err(DomainError::validation("unsupported locale")),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    #[default]
    Article,
    Category,
    Page,
    Home,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationProvenance {
    pub input_hash: String,
    pub generated_hash: String,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicContentMeta {
    pub schema_version: u32,
    pub id: Slug,
    pub locale: Locale,
    pub kind: ContentKind,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<PageKey>,
    #[serde(default, skip_serializing_if = "SectionPath::is_empty")]
    pub section_path: SectionPath,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    pub created: String,
    pub updated: String,
    /// Opaque digest used for source matching, never a private source path.
    pub source_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<TranslationProvenance>,
}

impl PublicContentMeta {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(DomainError::validation(
                "unsupported public Markdown schema",
            ));
        }
        Title::new(self.title.clone())?;
        Timestamp::new(self.created.clone())?;
        Timestamp::new(self.updated.clone())?;
        let valid_kind = match self.kind {
            ContentKind::Article | ContentKind::Category => {
                self.category.is_some() && self.page.is_none()
            }
            ContentKind::Page => {
                self.category.is_none() && self.page.as_ref().is_some_and(|p| p.as_str() == "about")
            }
            ContentKind::Home => self.category.is_none() && self.page.is_none(),
        };
        if !valid_kind {
            return Err(DomainError::validation(
                "invalid kind/category/page combination",
            ));
        }
        if self.kind != ContentKind::Article
            && (!self.tags.is_empty() || !self.section_path.is_empty())
        {
            return Err(DomainError::validation(
                "only articles may have tags or sections",
            ));
        }
        if self.section_path.segments().iter().any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || s.contains(['/', '\\'])
                || s.chars().any(char::is_control)
        }) {
            return Err(DomainError::validation("invalid public section"));
        }
        if self
            .tags
            .iter()
            .any(|s| s.trim().is_empty() || s.chars().any(char::is_control))
        {
            return Err(DomainError::validation("invalid tag identifier"));
        }
        if !is_digest(&self.source_hash) {
            return Err(DomainError::validation("invalid source fingerprint"));
        }
        if let Some(t) = &self.translation
            && (self.locale == Locale::Ja
                || !is_digest(&t.input_hash)
                || !is_digest(&t.generated_hash))
        {
            return Err(DomainError::validation("invalid translation provenance"));
        }
        Ok(())
    }

    /// Call validate before routing a deserialized document.
    pub fn path(&self) -> String {
        let path = match self.kind {
            ContentKind::Article => format!(
                "/{}/{}",
                self.category.map(|c| c.as_str()).unwrap_or_default(),
                self.id
            ),
            ContentKind::Category => {
                format!("/{}", self.category.map(|c| c.as_str()).unwrap_or_default())
            }
            ContentKind::Page => format!(
                "/{}",
                self.page.as_ref().map(PageKey::as_str).unwrap_or_default()
            ),
            ContentKind::Home => "/".into(),
        };
        self.locale.path(&path)
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
