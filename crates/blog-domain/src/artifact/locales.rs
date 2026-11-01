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
