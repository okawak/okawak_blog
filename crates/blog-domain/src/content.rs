//! Versioned, public Markdown frontmatter shared by export and publish.
//! Serialization formats and filesystem access belong to the callers.
use crate::{
    Category, DomainError, PageKey, Result, SectionPath, Sha256Digest, Slug, TagId, Timestamp,
    Title,
};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

pub const PUBLIC_CONTENT_SCHEMA_VERSION: u32 = 1;

/// SHA-256 hexadecimal prefix length shared by generated content IDs and heading anchors.
/// This does not restrict explicit Slugs or include anchor prefixes and suffixes.
pub const GENERATED_ID_HASH_LENGTH: usize = 12;

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    #[default]
    Ja,
    En,
    #[serde(rename = "zh-hans")]
    ZhHans,
}

impl Locale {
    pub const ALL: [Self; 3] = [Self::Ja, Self::En, Self::ZhHans];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ja => "ja",
            Self::En => "en",
            Self::ZhHans => "zh-hans",
        }
    }

    pub fn translations() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter().filter(|locale| *locale != Self::Ja)
    }

    pub fn path(self, route: &str) -> String {
        if self == Self::Ja {
            route.to_owned()
        } else {
            self.prefixed_path(route)
        }
    }

    pub fn artifact_key(self, key: &str) -> String {
        if self == Self::Ja {
            key.to_owned()
        } else {
            format!("{self}/{key}")
        }
    }

    fn prefixed_path(self, route: &str) -> String {
        format!("/{}{}", self.as_str(), route.trim_end_matches('/'))
    }

    pub fn split_path(path: &str) -> Option<(Self, &str)> {
        let first = path.strip_prefix('/')?.split('/').next()?;
        let locale = first.parse().ok()?;
        let rest = &path[first.len() + 1..];
        Some((locale, if rest.is_empty() { "/" } else { rest }))
    }

    /// Derive the storage path from the canonical URL. Only the locale home needs a basename.
    pub fn page_file(self, route: &str, extension: &str) -> String {
        let url = self.prefixed_path(route);
        let stem = url.trim_start_matches('/');
        let index = if route == "/" { "/index" } else { "" };
        format!("{stem}{index}.{extension}")
    }

    pub fn data_key(self, filename: &str) -> String {
        format!("{}/_data/{filename}", self.as_str())
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
        Self::ALL
            .into_iter()
            .find(|locale| locale.as_str() == value)
            .ok_or_else(|| DomainError::validation("unsupported locale"))
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
    pub input_hash: Sha256Digest,
    pub generated_hash: Sha256Digest,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicContentMeta {
    pub schema_version: u32,
    pub id: Slug,
    pub locale: Locale,
    pub kind: ContentKind,
    pub title: Title,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<PageKey>,
    #[serde(default, skip_serializing_if = "SectionPath::is_empty")]
    pub section_path: SectionPath,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    pub created: Timestamp,
    pub updated: Timestamp,
    /// Opaque digest used for source matching, never a private source path.
    pub source_hash: Sha256Digest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<TranslationProvenance>,
}

impl PublicContentMeta {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != PUBLIC_CONTENT_SCHEMA_VERSION {
            return Err(DomainError::validation(
                "unsupported public Markdown schema",
            ));
        }
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
        if self.translation.is_some() && self.locale == Locale::Ja {
            return Err(DomainError::validation("invalid translation provenance"));
        }
        Ok(())
    }

    /// Build the canonical route in a locale without copying the metadata.
    /// Call validate before routing a deserialized document.
    pub fn path(&self, locale: Locale) -> String {
        locale.path(&self.route_key())
    }

    pub fn route_key(&self) -> String {
        match self.kind {
            ContentKind::Article => {
                crate::build_article_path(&self.category.expect("validated article"), &self.id)
            }
            ContentKind::Category => {
                crate::build_category_path(&self.category.expect("validated category"))
            }
            ContentKind::Page => {
                crate::build_static_page_path(self.page.as_ref().expect("validated page"))
            }
            ContentKind::Home => crate::build_home_page_canonical_path().into(),
        }
    }

    pub fn markdown_path(&self, locale: Locale) -> String {
        locale.page_file(&self.route_key(), "md")
    }
}
