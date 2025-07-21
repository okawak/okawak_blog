use serde::Serialize;

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
