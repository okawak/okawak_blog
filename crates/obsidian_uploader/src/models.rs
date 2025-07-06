use serde::{Deserialize, Serialize};

/// Obsidianのフロントマター構造
#[derive(Debug, Deserialize, PartialEq)]
pub struct ObsidianFrontMatter {
    pub title: String,
    pub tags: Option<Vec<String>>,
    pub summary: Option<String>,
    pub is_completed: bool,
    pub priority: Option<i32>,
    pub created: String,
    pub updated: String,
    pub category: Option<String>, // 実際のファイルに存在するが、出力では使用しない
}

/// 出力用のフロントマター構造
#[derive(Debug, Serialize, PartialEq)]
pub struct OutputFrontMatter {
    pub title: String,
    pub tags: Option<Vec<String>>,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub created: String,
    pub updated: String,
    pub slug: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_obsidian_frontmatter_deserialization() {
        let yaml = r#"
title: "Test Article"
tags: ["rust", "programming"]
summary: "This is a test article"
is_completed: true
priority: 1
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-02T00:00:00+09:00"
category: "tech"
"#;

        let frontmatter: ObsidianFrontMatter = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(frontmatter.title, "Test Article");
        assert_eq!(
            frontmatter.tags,
            Some(vec!["rust".to_string(), "programming".to_string()])
        );
        assert_eq!(
            frontmatter.summary,
            Some("This is a test article".to_string())
        );
        assert_eq!(frontmatter.is_completed, true);
        assert_eq!(frontmatter.priority, Some(1));
        assert_eq!(frontmatter.created, "2025-01-01T00:00:00+09:00");
        assert_eq!(frontmatter.updated, "2025-01-02T00:00:00+09:00");
        assert_eq!(frontmatter.category, Some("tech".to_string()));
    }

    #[rstest]
    fn test_obsidian_frontmatter_minimal() {
        let yaml = r#"
title: "Minimal Article"
is_completed: false
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-01T00:00:00+09:00"
"#;

        let frontmatter: ObsidianFrontMatter = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(frontmatter.title, "Minimal Article");
        assert_eq!(frontmatter.tags, None);
        assert_eq!(frontmatter.summary, None);
        assert_eq!(frontmatter.is_completed, false);
        assert_eq!(frontmatter.priority, None);
        assert_eq!(frontmatter.category, None);
    }

    #[rstest]
    fn test_output_frontmatter_serialization() {
        let frontmatter = OutputFrontMatter {
            title: "Test Output".to_string(),
            tags: Some(vec!["test".to_string()]),
            description: Some("Test description".to_string()),
            priority: Some(1),
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-02T00:00:00+09:00".to_string(),
            slug: "abc123def456".to_string(),
        };

        let yaml = serde_yaml::to_string(&frontmatter).unwrap();

        assert!(yaml.contains("title: Test Output"));
        assert!(yaml.contains("slug: abc123def456"));
        assert!(yaml.contains("description: Test description"));
    }
}
