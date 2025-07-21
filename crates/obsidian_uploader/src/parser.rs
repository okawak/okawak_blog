use crate::error::{ObsidianError, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

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
    pub category: Option<String>,
}

/// parse Obsidian file, markdown file with frontmatter
pub fn parse_obsidian_file(path: impl AsRef<Path>) -> Result<Option<ObsidianFrontMatter>> {
    let content = fs::read_to_string(&path)?;
    parse_frontmatter(&content)
}

/// parse front matter from a string content
pub fn parse_frontmatter(content: &str) -> Result<Option<ObsidianFrontMatter>> {
    extract_yaml_frontmatter(content)?
        .map(|yaml| serde_yaml::from_str::<ObsidianFrontMatter>(yaml).map_err(Into::into))
        .transpose()
}

/// extract YAML frontmatter from the content
fn extract_yaml_frontmatter(text: &str) -> Result<Option<&str>> {
    let Some(rest) = text.trim_start().strip_prefix("---\n") else {
        return Ok(None);
    };

    match rest.split_once("\n---\n") {
        Some((yaml, _)) => Ok(Some(yaml)),
        None => Err(ObsidianError::ParseError(
            "unterminated front‑matter (closing `---` not found)".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use rstest::*;
    use tempfile::TempDir;

    #[rstest]
    #[case::full_frontmatter( indoc! {r#"
        title: "Test Article"
        tags: ["rust", "programming"]
        summary: "This is a test article"
        is_completed: true
        priority: 1
        created: "2025-01-01T00:00:00+09:00"
        updated: "2025-01-02T00:00:00+09:00"
        category: "tech"
        "#},
        "Test Article",
        true,
        Some(1),
        Some("tech")
    )]
    #[case::minimal_frontmatter(indoc! {r#"
        title: "Minimal Article"
        is_completed: false
        created: "2025-01-01T00:00:00+09:00"
        updated: "2025-01-01T00:00:00+09:00"
        "#},
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
    #[case::valid_frontmatter(
        indoc! {
            r#"
            ---
            title: "Test Article"
            tags: ["rust", "test"]
            is_completed: true
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-02T00:00:00+09:00"
            ---
            # Content
            "#},
        true,
        false
    )]
    #[case::no_frontmatter(
        indoc! {
            r#"
            # Article Without Frontmatter
            This article has no frontmatter.
            "#},
        false,
        false
    )]
    #[case::unterminated_frontmatter(
        indoc! {
            r#"
            ---
            tite: "Test Article"
            is_completed: true
            # Article Content
            "#},
        false,
        true
    )]
    #[case::invalid_yaml(
        indoc! {
            r#"
            ---
            invalid: yaml: content:
            ---
            # Content
            "#},
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
                assert!(frontmatter.is_completed);
            }
        }
    }

    #[rstest]
    #[case::valid_file(
        indoc! {
            r#"
            ---
            title: "File Test"
            is_completed: true
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            ---
            # Test Content
            "#},
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
            assert!(frontmatter.is_completed);
        }

        Ok(())
    }

    #[rstest]
    fn test_extract_yaml_frontmatter() -> Result<()> {
        let content = indoc! {
        r#"
        ---
        title: Test
        key: value
        ---

        Content here
        "#};

        let result = extract_yaml_frontmatter(content)?;
        assert!(result.is_some());

        let yaml = result.unwrap();
        assert!(yaml.contains("title: Test"));
        assert!(yaml.contains("key: value"));

        Ok(())
    }

    #[rstest]
    #[case::closing_at_eof(indoc! {r#"
        ---
        title: "File Test"
        is_completed: true
        created: "2025-01-01T00:00:00+09:00"
        updated: "2025-01-01T00:00:00+09:00"
        ---
        "#}, true, false)]
    #[case::no_newline_after_delim(indoc! {r#"
        ---
        title: "File Test"
        is_completed: true
        created: "2025-01-01T00:00:00+09:00"
        updated: "2025-01-01T00:00:00+09:00"
        ---# Heading
        "#}, true, true)]
    #[case::leading_blank_lines(indoc! {r#"


        ---
        title: "File Test"
        is_completed: true
        created: "2025-01-01T00:00:00+09:00"
        updated: "2025-01-01T00:00:00+09:00"
        ---

        body
        "#}, true, false)]
    #[case::empty_frontmatter(indoc! {r#"
        ---
        ---
        Body
        "#}, true, true)]
    fn test_additional_cases(
        #[case] content: &str,
        #[case] should_have_frontmatter: bool,
        #[case] should_error: bool,
    ) {
        let result = parse_frontmatter(content);

        assert_eq!(result.is_err(), should_error);
        if !should_error {
            assert_eq!(result.unwrap().is_some(), should_have_frontmatter);
        }
    }
}
