use crate::error::{ObsidianError, Result};
use crate::models::ObsidianFrontMatter;
use std::fs;
use std::path::Path;

/// Obsidianファイルからフロントマターを解析する
pub fn parse_obsidian_file<P: AsRef<Path>>(file_path: P) -> Result<Option<ObsidianFrontMatter>> {
    let content = fs::read_to_string(file_path.as_ref())?;
    parse_frontmatter(&content)
}

/// 文字列からフロントマターを解析する
pub fn parse_frontmatter(content: &str) -> Result<Option<ObsidianFrontMatter>> {
    // YAMLフロントマター（---で囲まれた部分）を抽出
    let frontmatter_content = extract_yaml_frontmatter(content)?;

    match frontmatter_content {
        Some(yaml_content) => {
            let frontmatter: ObsidianFrontMatter = serde_yaml::from_str(&yaml_content)?;
            Ok(Some(frontmatter))
        }
        None => Ok(None),
    }
}

/// YAMLフロントマターを抽出する
fn extract_yaml_frontmatter(content: &str) -> Result<Option<String>> {
    let content = content.trim_start();

    if !content.starts_with("---") {
        return Ok(None);
    }

    // 最初の---の後を探す
    let after_first_delimiter = &content[3..];

    // 改行があることを確認
    if !after_first_delimiter.starts_with('\n') && !after_first_delimiter.starts_with('\r') {
        return Ok(None);
    }

    // 2番目の---を探す
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 3 {
        return Ok(None);
    }

    let mut end_index = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_index = Some(i);
            break;
        }
    }

    match end_index {
        Some(end) => {
            let frontmatter_lines = &lines[1..end];
            let frontmatter_content = frontmatter_lines.join("\n");
            Ok(Some(frontmatter_content))
        }
        None => Err(ObsidianError::ParseError(
            "Unterminated YAML frontmatter (missing closing ---)".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_frontmatter_valid() -> Result<()> {
        let content = r#"---
title: "Test Article"
tags: ["rust", "test"]
summary: "A test article"
is_completed: true
priority: 1
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-02T00:00:00+09:00"
---

# Article Content

This is the article content.
"#;

        let result = parse_frontmatter(content)?;
        assert!(result.is_some());

        let frontmatter = result.unwrap();
        assert_eq!(frontmatter.title, "Test Article");
        assert_eq!(frontmatter.is_completed, true);
        assert_eq!(
            frontmatter.tags,
            Some(vec!["rust".to_string(), "test".to_string()])
        );

        Ok(())
    }

    #[test]
    fn test_parse_frontmatter_missing() -> Result<()> {
        let content = r#"# Article Without Frontmatter

This article has no frontmatter.
"#;

        let result = parse_frontmatter(content)?;
        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_parse_frontmatter_unterminated() {
        let content = r#"---
title: "Test Article"
is_completed: true
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-02T00:00:00+09:00"

# Article Content
"#;

        let result = parse_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_frontmatter_invalid_yaml() {
        let content = r#"---
title: "Test Article"
invalid: yaml: content:
is_completed: true
---

# Article Content
"#;

        let result = parse_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_obsidian_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");

        let content = r#"---
title: "File Test"
is_completed: true
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-01T00:00:00+09:00"
---

# Test Content
"#;

        fs::write(&file_path, content)?;

        let result = parse_obsidian_file(&file_path)?;
        assert!(result.is_some());

        let frontmatter = result.unwrap();
        assert_eq!(frontmatter.title, "File Test");
        assert_eq!(frontmatter.is_completed, true);

        Ok(())
    }

    #[test]
    fn test_extract_yaml_frontmatter() -> Result<()> {
        let content = r#"---
title: Test
key: value
---

Content here
"#;

        let result = extract_yaml_frontmatter(content)?;
        assert!(result.is_some());

        let yaml = result.unwrap();
        assert!(yaml.contains("title: Test"));
        assert!(yaml.contains("key: value"));
        assert!(!yaml.contains("---"));
        assert!(!yaml.contains("Content here"));

        Ok(())
    }
}
