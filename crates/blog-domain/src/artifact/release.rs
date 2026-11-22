use jiff::tz::Offset;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

pub const ARTIFACT_RELEASE_SCHEMA_VERSION: u32 = 1;

/// Points readers at one immutable set of published artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactReleasePointerDocument {
    pub schema_version: u32,
    pub release_id: String,
    pub artifact_prefix: String,
    pub publisher_commit: String,
    pub source_commit: String,
    /// Public repository revision containing the reviewed Markdown.
    pub content_commit: String,
    pub generated_at: String,
}

impl ArtifactReleasePointerDocument {
    pub fn generated_at_time(&self) -> crate::Result<SystemTime> {
        let invalid = || crate::DomainError::validation("generated_at");
        let (datetime, offset) = crate::rfc3339::parse(&self.generated_at)
            .filter(|(_, offset)| *offset == Offset::UTC)
            .ok_or_else(invalid)?;
        let timestamp = offset.to_timestamp(datetime).map_err(|_| invalid())?;
        let time = SystemTime::try_from(timestamp).map_err(|_| invalid())?;
        // Jiff clamps second 60 to 59. Preserve the legacy SystemTime for leap seconds.
        if self.generated_at.as_bytes().get(17..19) == Some(b"60") {
            time.checked_add(Duration::from_secs(1)).ok_or_else(invalid)
        } else {
            Ok(time)
        }
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
        if self.content_commit.len() != 40
            || !self.content_commit.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(crate::DomainError::validation("content_commit"));
        }
        self.generated_at_time()?;
        Ok(())
    }
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
            content_commit: "a".repeat(40),
            generated_at: "2026-07-12T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn release_requires_a_valid_public_content_revision() {
        let mut current = release_pointer("releases/one/site");
        assert!(current.validate().is_ok());
        let encoded = serde_json::to_value(&current).unwrap();
        assert_eq!(encoded["source_commit"], "source-sha");
        // Length and hexadecimal content are independent requirements.
        for invalid in ["a".repeat(39), "a".repeat(41), "g".repeat(40)] {
            current.content_commit = invalid;
            assert_eq!(
                current.validate(),
                Err(crate::DomainError::validation("content_commit"))
            );
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

        std::assert_matches!(
            pointer.validate(),
            Err(crate::DomainError::ValidationError { field }) if field == "schema_version"
        );
    }

    #[test]
    fn artifact_release_pointer_rejects_unsafe_prefixes() {
        for prefix in ["", "/releases/one", "releases/../site", "releases/one/"] {
            std::assert_matches!(
                release_pointer(prefix).validate(),
                Err(crate::DomainError::ValidationError { field }) if field == "artifact_prefix"
            );
        }
    }

    #[test]
    fn artifact_release_pointer_requires_rfc3339_utc_generated_at() {
        for generated_at in [
            "not-a-timestamp",
            "2026-07-12T12:00:00",
            "2026-07-12T21:00:00+09:00",
            "2026-07-12T12:00:00Z[UTC]",
        ] {
            let mut pointer = release_pointer("releases/release-123/site");
            pointer.generated_at = generated_at.to_string();

            std::assert_matches!(
                pointer.validate(),
                Err(crate::DomainError::ValidationError { field }) if field == "generated_at"
            );
        }

        let mut pointer = release_pointer("releases/release-123/site");
        for offset in ["Z", "z", "+00:00", "-00:00"] {
            pointer.generated_at = format!("1969-12-31T23:59:59.123456789123{offset}");
            assert_eq!(
                pointer.generated_at_time().unwrap(),
                SystemTime::UNIX_EPOCH - std::time::Duration::from_nanos(876_543_211)
            );
        }
        pointer.generated_at = "2016-12-31T23:59:60.5Z".to_string();
        assert_eq!(
            pointer.generated_at_time().unwrap(),
            SystemTime::UNIX_EPOCH + std::time::Duration::new(1_483_228_800, 500_000_000)
        );
    }
}
