use crate::error::{ObsidianError, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

/// SHA-256ハッシュベースのslugを生成する
pub fn generate_slug<P: AsRef<Path>>(title: &str, relative_path: P, created: &str) -> Result<String> {
    let relative_path_str = relative_path.as_ref()
        .to_str()
        .ok_or_else(|| ObsidianError::PathError("Invalid path encoding".to_string()))?;
    
    // ハッシュ生成元文字列
    let hash_input = format!("{}/{}/{}", title, relative_path_str, created);
    
    // SHA-256ハッシュ計算
    let mut hasher = Sha256::new();
    hasher.update(hash_input.as_bytes());
    let hash_result = hasher.finalize();
    
    // 先頭12文字（6バイト）を16進文字列として使用
    let slug = format!("{:x}", hash_result)
        .chars()
        .take(12)
        .collect::<String>();
    
    Ok(slug)
}

/// slugの一意性を検証する（将来の拡張用）
pub fn validate_slug_uniqueness(slug: &str, existing_slugs: &[String]) -> bool {
    !existing_slugs.contains(&slug.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_generate_slug() -> Result<()> {
        let title = "Test Article";
        let relative_path = PathBuf::from("tech/rust/test.md");
        let created = "2025-01-01T00:00:00+09:00";
        
        let slug = generate_slug(title, &relative_path, created)?;
        
        // slugの長さは12文字
        assert_eq!(slug.len(), 12);
        
        // 16進文字のみ含む
        assert!(slug.chars().all(|c| c.is_ascii_hexdigit()));
        
        Ok(())
    }
    
    #[test]
    fn test_generate_slug_deterministic() -> Result<()> {
        let title = "Same Title";
        let relative_path = PathBuf::from("same/path.md");
        let created = "2025-01-01T00:00:00+09:00";
        
        let slug1 = generate_slug(title, &relative_path, created)?;
        let slug2 = generate_slug(title, &relative_path, created)?;
        
        // 同じ入力に対して同じslugが生成される
        assert_eq!(slug1, slug2);
        
        Ok(())
    }
    
    #[test]
    fn test_generate_slug_different_inputs() -> Result<()> {
        let relative_path = PathBuf::from("test/path.md");
        let created = "2025-01-01T00:00:00+09:00";
        
        let slug1 = generate_slug("Title 1", &relative_path, created)?;
        let slug2 = generate_slug("Title 2", &relative_path, created)?;
        
        // 異なる入力に対して異なるslugが生成される
        assert_ne!(slug1, slug2);
        
        Ok(())
    }
    
    #[test]
    fn test_generate_slug_japanese_title() -> Result<()> {
        let title = "日本語のタイトル";
        let relative_path = PathBuf::from("tech/article.md");
        let created = "2025-01-01T00:00:00+09:00";
        
        let slug = generate_slug(title, &relative_path, created)?;
        
        // 日本語タイトルでも正常にslugが生成される
        assert_eq!(slug.len(), 12);
        assert!(slug.chars().all(|c| c.is_ascii_hexdigit()));
        
        Ok(())
    }
    
    #[test]
    fn test_validate_slug_uniqueness() {
        let existing_slugs = vec![
            "abc123def456".to_string(),
            "789xyz012tuv".to_string(),
        ];
        
        // 既存のslugは一意ではない
        assert!(!validate_slug_uniqueness("abc123def456", &existing_slugs));
        
        // 新しいslugは一意
        assert!(validate_slug_uniqueness("new123slug45", &existing_slugs));
    }
    
    #[test]
    fn test_slug_collision_resistance() -> Result<()> {
        // 似たような入力でも異なるslugが生成されることを確認
        let base_path = PathBuf::from("tech/test.md");
        let created = "2025-01-01T00:00:00+09:00";
        
        let slug1 = generate_slug("Test", &base_path, created)?;
        let slug2 = generate_slug("Test ", &base_path, created)?; // 末尾にスペース
        let slug3 = generate_slug("test", &base_path, created)?; // 小文字
        
        assert_ne!(slug1, slug2);
        assert_ne!(slug1, slug3);
        assert_ne!(slug2, slug3);
        
        Ok(())
    }
}