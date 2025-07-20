use crate::error::{ObsidianError, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

/// SHA-256ハッシュベースのslugを生成する
pub fn generate_slug<P: AsRef<Path>>(
    title: &str,
    relative_path: P,
    created: &str,
) -> Result<String> {
    let relative_path_str = relative_path
        .as_ref()
        .to_str()
        .ok_or_else(|| ObsidianError::PathError("Invalid path encoding".to_string()))?;

    // ハッシュ生成元文字列
    let hash_input = format!("{title}/{relative_path_str}/{created}");

    // SHA-256ハッシュ計算
    let mut hasher = Sha256::new();
    hasher.update(hash_input.as_bytes());
    let hash_result = hasher.finalize();

    // 先頭6バイト（12文字の16進文字列）を使用してslugを生成
    let slug = hash_result[..6]
        .iter()
        .fold(String::with_capacity(12), |mut acc, byte| {
            acc.push_str(&format!("{byte:02x}"));
            acc
        });

    Ok(slug)
}

/// slugの一意性を検証する（将来の拡張用）
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

        // 同じ入力では同じslugが生成される
        let slug1 = generate_slug("Same Title", &relative_path, created)?;
        let slug2 = generate_slug("Same Title", &relative_path, created)?;
        assert_eq!(slug1, slug2);

        // 異なる入力では異なるslugが生成される
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
