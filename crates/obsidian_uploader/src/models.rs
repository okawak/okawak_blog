use serde::{Deserialize, Serialize};

/// Obsidianのフロントマター構造
#[derive(Debug, Deserialize, PartialEq)]
pub struct ObsidianFrontMatter {
    pub title: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    pub summary: Option<String>,
    pub is_completed: bool,
    pub priority: Option<i32>,
    pub created: String,
    pub updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>, // 実際のファイルに存在するが、出力では使用しない
}

/// 出力用のフロントマター構造
#[derive(Debug, Serialize, PartialEq)]
pub struct OutputFrontMatter {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[case::full_frontmatter(
        r#"title: "Test Article"
tags: ["rust", "programming"]
summary: "This is a test article"
is_completed: true
priority: 1
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-02T00:00:00+09:00"
category: "tech""#,
        "Test Article",
        true,
        Some(1),
        Some("tech")
    )]
    #[case::minimal_frontmatter(
        r#"title: "Minimal Article"
is_completed: false
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-01T00:00:00+09:00""#,
        "Minimal Article",
        false,
        None,
        None
    )]
    fn test_obsidian_frontmatter_deserialization(
        #[case] yaml: &str,
        #[case] expected_title: &str,
        #[case] expected_completed: bool,
        #[case] expected_priority: Option<i32>,
        #[case] expected_category: Option<&str>,
    ) {
        let frontmatter: ObsidianFrontMatter = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(frontmatter.title, expected_title);
        assert_eq!(frontmatter.is_completed, expected_completed);
        assert_eq!(frontmatter.priority, expected_priority);
        assert_eq!(
            frontmatter.category,
            expected_category.map(|s| s.to_string())
        );
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
