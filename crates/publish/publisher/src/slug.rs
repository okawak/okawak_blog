use crate::error::{ObsidianError, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Generates a SHA-256-based slug.
pub fn generate_slug<P: AsRef<Path>>(
    title: &str,
    relative_path: P,
    created: &str,
) -> Result<String> {
    let relative_path_str = relative_path
        .as_ref()
        .to_str()
        .ok_or_else(|| ObsidianError::Path("Invalid path encoding".to_string()))?;

    // Input string used to build the hash.
    let hash_input = format!("{title}/{relative_path_str}/{created}");

    // Compute the SHA-256 hash.
    let mut hasher = Sha256::new();
    hasher.update(hash_input.as_bytes());
    let hash_result = hasher.finalize();

    // Use the first 6 bytes to build a 12-character hex slug.
    let slug = hash_result[..6]
        .iter()
        .fold(String::with_capacity(12), |mut acc, byte| {
            acc.push_str(&format!("{byte:02x}"));
            acc
        });

    Ok(slug)
}

/// Validates slug uniqueness for future extensions.
pub fn validate_slug_uniqueness(slug: &str, existing_slugs: &[String]) -> bool {
    !existing_slugs.contains(&slug.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::path::PathBuf;

    #[rstest]
    #[case::english_title("Test Article", "tech/rust/test.md")]
    #[case::japanese_title("日本語のタイトル", "tech/article.md")]
    #[case::symbols_title("Test & Special Characters", "daily/test.md")]
    fn test_generate_slug_formats(#[case] title: &str, #[case] path: &str) -> Result<()> {
        let relative_path = PathBuf::from(path);
        let created = "2025-01-01T00:00:00+09:00";

        let slug = generate_slug(title, &relative_path, created)?;

        assert_eq!(slug.len(), 12);
        assert!(slug.chars().all(|c| c.is_ascii_hexdigit()));

        Ok(())
    }

    #[rstest]
    fn test_generate_slug_deterministic_and_unique() -> Result<()> {
        let relative_path = PathBuf::from("test/path.md");
        let created = "2025-01-01T00:00:00+09:00";

        // The same input should always produce the same slug.
        let slug1 = generate_slug("Same Title", &relative_path, created)?;
        let slug2 = generate_slug("Same Title", &relative_path, created)?;
        assert_eq!(slug1, slug2);

        // Different inputs should produce different slugs.
        let slug3 = generate_slug("Different Title", &relative_path, created)?;
        assert_ne!(slug1, slug3);

        Ok(())
    }

    #[rstest]
    #[case::existing_slug("abc123def456", false)]
    #[case::new_slug("new123slug45", true)]
    fn test_validate_slug_uniqueness(#[case] slug: &str, #[case] should_be_unique: bool) {
        let existing_slugs = vec!["abc123def456".to_string(), "789xyz012tuv".to_string()];
        assert_eq!(
            validate_slug_uniqueness(slug, &existing_slugs),
            should_be_unique
        );
    }

    #[rstest]
    #[case::exact_match("Test", true)]
    #[case::trailing_space("Test ", false)]
    #[case::lowercase("test", false)]
    fn test_slug_collision_resistance(
        #[case] title: &str,
        #[case] should_match: bool,
    ) -> Result<()> {
        let base_path = PathBuf::from("tech/test.md");
        let created = "2025-01-01T00:00:00+09:00";
        let base_slug = generate_slug("Test", &base_path, created)?;
        let test_slug = generate_slug(title, &base_path, created)?;

        assert_eq!(base_slug == test_slug, should_match);
        Ok(())
    }
}
