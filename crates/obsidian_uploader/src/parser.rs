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

    if !content.starts_with("---\n") && !content.starts_with("---\r") {
        return Ok(None);
    }

    let lines: Vec<&str> = content.lines().collect();
    let end_pos = lines
        .iter()
        .skip(1)
        .position(|&line| line.trim() == "---")
        .ok_or_else(|| {
            ObsidianError::ParseError(
                "Unterminated YAML frontmatter (missing closing ---)".to_string(),
            )
        })?;

    let frontmatter = lines[1..end_pos + 1].join("\n");
    Ok((!frontmatter.is_empty()).then_some(frontmatter))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::fs;
    use tempfile::TempDir;

    #[rstest]
    #[case::valid_frontmatter(
        r#"---
title: "Test Article"
tags: ["rust", "test"]
is_completed: true
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-02T00:00:00+09:00"
---
# Content"#,
        true,
        false
    )]
    #[case::no_frontmatter(
        r#"# Article Without Frontmatter
This article has no frontmatter."#,
        false,
        false
    )]
    #[case::unterminated_frontmatter(
        r#"---
title: "Test Article"
is_completed: true
# Article Content"#,
        false,
        true
    )]
    #[case::invalid_yaml(
        r#"---
invalid: yaml: content:
---
# Content"#,
        false,
        true
    )]
    fn test_parse_frontmatter(
        #[case] content: &str,
        #[case] should_have_frontmatter: bool,
        #[case] should_error: bool,
    ) {
        let result = parse_frontmatter(content);

        if should_error {
            assert!(result.is_err());
        } else {
            let result = result.unwrap();
            assert_eq!(result.is_some(), should_have_frontmatter);

            if should_have_frontmatter {
                let frontmatter = result.unwrap();
                assert_eq!(frontmatter.title, "Test Article");
                assert_eq!(frontmatter.is_completed, true);
            }
        }
    }

    #[rstest]
    #[case::valid_file(
        r#"---
title: "File Test"
is_completed: true
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-01T00:00:00+09:00"
---
# Test Content"#,
        true
    )]
    #[case::no_frontmatter_file("# Just content", false)]
    fn test_parse_obsidian_file(
        #[case] content: &str,
        #[case] should_have_frontmatter: bool,
    ) -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        fs::write(&file_path, content)?;

        let result = parse_obsidian_file(&file_path)?;
        assert_eq!(result.is_some(), should_have_frontmatter);

        if should_have_frontmatter {
            let frontmatter = result.unwrap();
            assert_eq!(frontmatter.title, "File Test");
            assert_eq!(frontmatter.is_completed, true);
        }

        Ok(())
    }

    #[rstest]
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

        Ok(())
    }
}
