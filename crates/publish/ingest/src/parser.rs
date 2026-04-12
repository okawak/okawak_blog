use crate::error::{IngestError, Result};
use domain::ContentKind;
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct ObsidianFrontMatter {
    pub title: String,
    #[serde(default)]
    pub kind: ContentKind,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    pub summary: Option<String>,
    pub is_completed: bool,
    pub priority: Option<i32>,
    pub created: String,
    pub updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParsedObsidianFile {
    pub front_matter: ObsidianFrontMatter,
    pub markdown_body: String,
}

#[derive(Debug, PartialEq, Eq)]
enum FrontmatterSplit<'a> {
    NoFrontmatter,
    Complete { yaml: &'a str, body: &'a str },
    Unterminated,
}

/// Parse Obsidian file and return the frontmatter plus markdown body.
pub fn parse_obsidian_file(path: impl AsRef<Path>) -> Result<Option<ParsedObsidianFile>> {
    let content = fs::read_to_string(&path)?;
    match split_frontmatter(&content) {
        FrontmatterSplit::Complete { yaml, body } => {
            let front_matter = serde_yaml::from_str::<ObsidianFrontMatter>(yaml)?;
            Ok(Some(ParsedObsidianFile {
                front_matter,
                markdown_body: normalize_markdown_body(body),
            }))
        }
        FrontmatterSplit::NoFrontmatter => Ok(None),
        FrontmatterSplit::Unterminated => Err(unterminated_frontmatter_error()),
    }
}

fn split_frontmatter(content: &str) -> FrontmatterSplit<'_> {
    let trimmed = content.trim_start();
    let Some(rest) = trimmed.strip_prefix("---\n") else {
        return FrontmatterSplit::NoFrontmatter;
    };

    match rest.split_once("\n---\n") {
        Some((yaml, body)) => FrontmatterSplit::Complete { yaml, body },
        None => FrontmatterSplit::Unterminated,
    }
}

fn normalize_markdown_body(body: &str) -> String {
    body.trim_end_matches(['\r', '\n']).to_string()
}

fn unterminated_frontmatter_error() -> IngestError {
    IngestError::Parse("unterminated front‑matter (closing `---` not found)".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use rstest::*;
    use tempfile::TempDir;

    fn extract_yaml_frontmatter(text: &str) -> Result<Option<&str>> {
        match split_frontmatter(text) {
            FrontmatterSplit::Complete { yaml, .. } => Ok(Some(yaml)),
            FrontmatterSplit::NoFrontmatter => Ok(None),
            FrontmatterSplit::Unterminated => Err(unterminated_frontmatter_error()),
        }
    }

    fn parse_frontmatter(content: &str) -> Result<Option<ObsidianFrontMatter>> {
        extract_yaml_frontmatter(content)?
            .map(|yaml| serde_yaml::from_str::<ObsidianFrontMatter>(yaml).map_err(Into::into))
            .transpose()
    }

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
        assert_eq!(frontmatter.kind, ContentKind::Article);
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
            let parsed_file = result.unwrap();
            assert_eq!(parsed_file.front_matter.title, "File Test");
            assert_eq!(parsed_file.front_matter.kind, ContentKind::Article);
            assert!(parsed_file.front_matter.is_completed);
            assert_eq!(parsed_file.markdown_body, "# Test Content");
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

    #[rstest]
    fn test_parse_explicit_kind() {
        let content = indoc! {
            r#"
            ---
            title: "About"
            kind: page
            page: about
            is_completed: true
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            ---
            About body
            "#
        };

        let frontmatter = parse_frontmatter(content).unwrap().unwrap();

        assert_eq!(frontmatter.kind, ContentKind::Page);
        assert_eq!(frontmatter.page.as_deref(), Some("about"));
    }

    #[rstest]
    #[case::with_frontmatter(
        "---\ntitle: Test\n---\n# Content\n\nBody text",
        FrontmatterSplit::Complete { yaml: "title: Test", body: "# Content\n\nBody text" }
    )]
    #[case::no_frontmatter("# Content\n\nBody text", FrontmatterSplit::NoFrontmatter)]
    #[case::malformed_frontmatter(
        "---\ntitle: Test\n# Content\n\nBody text",
        FrontmatterSplit::Unterminated
    )]
    #[case::empty_body(
        "---\ntitle: Test\n---\n",
        FrontmatterSplit::Complete { yaml: "title: Test", body: "" }
    )]
    #[case::whitespace_handling(
        "   ---\ntitle: Test\n---\n\n# Content",
        FrontmatterSplit::Complete { yaml: "title: Test", body: "\n# Content" }
    )]
    #[case::multiple_frontmatter_separators(
        "---\ntitle: Test\n---\n# Section\n---\nMore content",
        FrontmatterSplit::Complete { yaml: "title: Test", body: "# Section\n---\nMore content" }
    )]
    #[case::frontmatter_with_complex_yaml(
        "---\ntitle: \"Complex: Title\"\ntags: [\"tag1\", \"tag2\"]\n---\n## Heading",
        FrontmatterSplit::Complete {
            yaml: "title: \"Complex: Title\"\ntags: [\"tag1\", \"tag2\"]",
            body: "## Heading"
        }
    )]
    fn test_split_frontmatter(#[case] input: &str, #[case] expected: FrontmatterSplit<'_>) {
        let result = split_frontmatter(input);
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::body_with_trailing_newline(
        "---\ntitle: Test\nis_completed: true\ncreated: \"2025-01-01T00:00:00+09:00\"\nupdated: \"2025-01-01T00:00:00+09:00\"\n---\n# Content\n",
        "# Content"
    )]
    #[case::body_with_blank_line(
        "---\ntitle: Test\nis_completed: true\ncreated: \"2025-01-01T00:00:00+09:00\"\nupdated: \"2025-01-01T00:00:00+09:00\"\n---\n# Content\n\nBody text",
        "# Content\n\nBody text"
    )]
    fn test_parse_obsidian_file_normalizes_markdown_body(
        #[case] content: &str,
        #[case] expected_body: &str,
    ) -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("normalize.md");
        fs::write(&file_path, content)?;

        let parsed_file = parse_obsidian_file(&file_path)?.unwrap();

        assert_eq!(parsed_file.markdown_body, expected_body);
        Ok(())
    }
}
