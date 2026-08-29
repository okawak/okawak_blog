use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub const ARTIFACT_RELEASE_SCHEMA_VERSION: u32 = 1;

/// Points readers at one immutable set of published artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactReleasePointerDocument {
    pub schema_version: u32,
    pub release_id: String,
    pub artifact_prefix: String,
    pub publisher_commit: String,
    pub source_commit: String,
    pub generated_at: String,
}

impl ArtifactReleasePointerDocument {
    pub fn generated_at_time(&self) -> crate::Result<SystemTime> {
        parse_rfc3339_utc(&self.generated_at)
            .map(|timestamp| timestamp.to_utc().into())
            .ok_or_else(|| crate::DomainError::validation("generated_at"))
    }

    pub fn validate(&self) -> crate::Result<()> {
        if self.schema_version != ARTIFACT_RELEASE_SCHEMA_VERSION {
            return Err(crate::DomainError::validation("schema_version"));
        }
        if !is_safe_path_segment(&self.release_id) {
            return Err(crate::DomainError::validation("release_id"));
        }
        if !is_safe_relative_prefix(&self.artifact_prefix) {
            return Err(crate::DomainError::validation("artifact_prefix"));
        }
        for (field, value) in [
            ("publisher_commit", self.publisher_commit.as_str()),
            ("source_commit", self.source_commit.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(crate::DomainError::validation(field));
            }
        }
        self.generated_at_time()?;
        Ok(())
    }
}

fn parse_rfc3339_utc(value: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .filter(|timestamp| timestamp.offset().local_minus_utc() == 0)
}

fn is_safe_path_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
}

fn is_safe_relative_prefix(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains('\\')
        && value.split('/').all(is_safe_path_segment)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_pointer(prefix: &str) -> ArtifactReleasePointerDocument {
        ArtifactReleasePointerDocument {
            schema_version: ARTIFACT_RELEASE_SCHEMA_VERSION,
            release_id: "release-123".to_string(),
            artifact_prefix: prefix.to_string(),
            publisher_commit: "publisher-sha".to_string(),
            source_commit: "source-sha".to_string(),
            generated_at: "2026-07-12T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn artifact_release_pointer_accepts_immutable_relative_prefix() {
        assert!(
            release_pointer("releases/release-123/site")
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn artifact_release_pointer_rejects_unsupported_schema() {
        let mut pointer = release_pointer("releases/release-123/site");
        pointer.schema_version += 1;

        assert!(matches!(
            pointer.validate(),
            Err(crate::DomainError::ValidationError { field }) if field == "schema_version"
        ));
    }

    #[test]
    fn artifact_release_pointer_rejects_unsafe_prefixes() {
        for prefix in ["", "/releases/one", "releases/../site", "releases/one/"] {
            assert!(matches!(
                release_pointer(prefix).validate(),
                Err(crate::DomainError::ValidationError { field }) if field == "artifact_prefix"
            ));
        }
    }

    #[test]
    fn artifact_release_pointer_requires_rfc3339_utc_generated_at() {
        for generated_at in [
            "not-a-timestamp",
            "2026-07-12T12:00:00",
            "2026-07-12T21:00:00+09:00",
        ] {
            let mut pointer = release_pointer("releases/release-123/site");
            pointer.generated_at = generated_at.to_string();

            assert!(matches!(
                pointer.validate(),
                Err(crate::DomainError::ValidationError { field }) if field == "generated_at"
            ));
        }

        let mut pointer = release_pointer("releases/release-123/site");
        pointer.generated_at = "2026-07-12T12:00:00+00:00".to_string();
        assert!(pointer.validate().is_ok());
    }
}
