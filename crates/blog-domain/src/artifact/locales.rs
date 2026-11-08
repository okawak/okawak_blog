use crate::{DomainError, Locale, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Available public paths in one immutable release. Keys use Japanese paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteLocalesDocument {
    pub schema_version: u32,
    pub routes: BTreeMap<String, Vec<Locale>>,
}

impl Default for SiteLocalesDocument {
    fn default() -> Self {
        Self {
            schema_version: 1,
            routes: BTreeMap::new(),
        }
    }
}

impl SiteLocalesDocument {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || self.routes.iter().any(|(path, locales)| {
                !path.starts_with('/')
                    || path.starts_with("//")
                    || path.contains(['\\', '?', '#'])
                    || path.split('/').any(|p| p == "." || p == "..")
                    || locales.is_empty()
                    || locales.len() > 2
                    || (locales.len() == 2 && locales[0] == locales[1])
            })
        {
            return Err(DomainError::validation("invalid site locale routes"));
        }
        Ok(())
    }

    pub fn path(&self, japanese_path: &str, locale: Locale) -> Option<String> {
        self.routes
            .get(japanese_path)
            .filter(|locales| locales.contains(&locale))
            .map(|_| locale.path(japanese_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(path: &str, locales: Vec<Locale>) -> SiteLocalesDocument {
        SiteLocalesDocument {
            routes: [(path.into(), locales)].into(),
            ..Default::default()
        }
    }

    #[test]
    fn validates_supported_schema_even_without_routes() {
        SiteLocalesDocument::default().validate().unwrap();
        for schema_version in [0, 2] {
            assert!(
                SiteLocalesDocument {
                    schema_version,
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
    }

    #[test]
    fn accepts_absolute_public_paths_with_one_or_two_distinct_locales() {
        for path in ["/", "/about", "/tech/article", "/tech/dot.name"] {
            for locales in [
                vec![Locale::Ja],
                vec![Locale::En],
                vec![Locale::Ja, Locale::En],
                vec![Locale::En, Locale::Ja],
            ] {
                document(path, locales).validate().unwrap();
            }
        }
    }

    #[test]
    fn rejects_each_unsafe_route_component() {
        for path in [
            "",
            "about",
            "//example.com",
            "/tech\\article",
            "/about?lang=en",
            "/about#title",
            "/.",
            "/..",
            "/tech/./article",
            "/tech/../article",
        ] {
            assert!(
                document(path, vec![Locale::Ja]).validate().is_err(),
                "accepted {path:?}"
            );
        }
    }

    #[test]
    fn rejects_empty_duplicate_and_oversized_locale_lists() {
        for locales in [
            vec![],
            vec![Locale::Ja, Locale::Ja],
            vec![Locale::En, Locale::En],
            vec![Locale::Ja, Locale::En, Locale::Ja],
        ] {
            assert!(document("/about", locales).validate().is_err());
        }
    }

    #[test]
    fn resolves_only_paths_available_in_the_requested_locale() {
        let document = SiteLocalesDocument {
            routes: [
                ("/".into(), vec![Locale::Ja, Locale::En]),
                ("/about".into(), vec![Locale::Ja]),
                ("/tech/article".into(), vec![Locale::Ja, Locale::En]),
                ("/daily/english".into(), vec![Locale::En]),
            ]
            .into(),
            ..Default::default()
        };
        document.validate().unwrap();
        for (path, locale, expected) in [
            ("/", Locale::Ja, Some("/")),
            ("/", Locale::En, Some("/en")),
            ("/about", Locale::Ja, Some("/about")),
            ("/about", Locale::En, None),
            ("/tech/article", Locale::Ja, Some("/tech/article")),
            ("/tech/article", Locale::En, Some("/en/tech/article")),
            ("/daily/english", Locale::Ja, None),
            ("/daily/english", Locale::En, Some("/en/daily/english")),
            ("/missing", Locale::Ja, None),
            ("/missing", Locale::En, None),
        ] {
            assert_eq!(document.path(path, locale).as_deref(), expected);
        }
    }
}
