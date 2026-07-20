use super::error::{IngestError, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub(crate) struct ObsidianFrontMatter {
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) kind: ContentKind,
    #[serde(default)]
    pub(crate) tags: Option<Vec<String>>,
    pub(crate) summary: Option<String>,
    pub(crate) is_completed: bool,
    pub(crate) priority: Option<i32>,
    pub(crate) created: String,
    pub(crate) updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) page: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct ParsedObsidianFile {
    pub(crate) front_matter: ObsidianFrontMatter,
    pub(crate) markdown_body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ContentKind {
    #[default]
    Article,
    Category,
    Page,
    Home,
}

#[derive(Debug, PartialEq, Eq)]
enum FrontmatterSplit<'a> {
    NoFrontmatter,
    Complete { yaml: &'a str, body: &'a str },
}

/// Parse Obsidian file and return the frontmatter plus markdown body.
pub(crate) fn parse_obsidian_file(path: impl AsRef<Path>) -> Result<Option<ParsedObsidianFile>> {
    let content = fs::read_to_string(&path)?;
    match split_frontmatter(&content)? {
        FrontmatterSplit::Complete { yaml, body } => {
            let front_matter = serde_yaml::from_str::<ObsidianFrontMatter>(yaml)?;
            Ok(Some(ParsedObsidianFile {
                front_matter,
                markdown_body: body.to_owned(),
            }))
        }
        FrontmatterSplit::NoFrontmatter => Ok(None),
    }
}

fn split_frontmatter(content: &str) -> Result<FrontmatterSplit<'_>> {
    let trimmed = content.trim_start();
    let Some(rest) = trimmed.strip_prefix("---\n") else {
        return Ok(FrontmatterSplit::NoFrontmatter);
    };

    match rest.split_once("\n---\n") {
        Some((yaml, body)) => Ok(FrontmatterSplit::Complete { yaml, body }),
        None => Err(IngestError::Parse(
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

    fn extract_yaml_frontmatter(text: &str) -> Result<Option<&str>> {
        match split_frontmatter(text)? {
            FrontmatterSplit::Complete { yaml, .. } => Ok(Some(yaml)),
            FrontmatterSplit::NoFrontmatter => Ok(None),
        }
    }

    fn parse_frontmatter(content: &str) -> Result<Option<ObsidianFrontMatter>> {
        extract_yaml_frontmatter(content)?
            .map(|yaml| serde_yaml::from_str::<ObsidianFrontMatter>(yaml).map_err(Into::into))
            .transpose()
    }

    #[rstest]
    #[case::full_frontmatter(
        indoc! {r#"
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
    #[case::minimal_frontmatter(
        indoc! {r#"
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
        indoc! {r#"
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
        indoc! {r#"
            # Article Without Frontmatter
            This article has no frontmatter.
        "#},
        false,
        false
    )]
    #[case::unterminated_frontmatter(
        indoc! {r#"
            ---
            tite: "Test Article"
            is_completed: true
            # Article Content
        "#},
        false,
        true
    )]
    #[case::invalid_yaml(
        indoc! {r#"
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
        indoc! {r#"
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
            assert_eq!(
                parsed_file.markdown_body,
                indoc! {r#"
                    # Test Content
                "#}
            );
        }

        Ok(())
    }

    #[rstest]
    fn test_extract_yaml_frontmatter() -> Result<()> {
        let content = indoc! {r#"
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
    #[case::closing_at_eof(
        indoc! {r#"
            ---
            title: "File Test"
            is_completed: true
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            ---
        "#},
        true,
        false
    )]
    #[case::no_newline_after_delim(
        indoc! {r#"
            ---
            title: "File Test"
            is_completed: true
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            ---# Heading
        "#},
        true,
        true
    )]
    #[case::leading_blank_lines(
        indoc! {r#"


            ---
            title: "File Test"
            is_completed: true
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            ---

            body
        "#},
        true,
        false
    )]
    #[case::empty_frontmatter(
        indoc! {r#"
            ---
            ---
            Body
        "#},
        true,
        true
    )]
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
        let content = indoc! {r#"
            ---
            title: "About"
            kind: page
            page: about
            is_completed: true
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            ---
            About body
        "#};

        let frontmatter = parse_frontmatter(content).unwrap().unwrap();

        assert_eq!(frontmatter.kind, ContentKind::Page);
        assert_eq!(frontmatter.page.as_deref(), Some("about"));
    }

    #[rstest]
    #[case::with_frontmatter(
        indoc! {r#"
            ---
            title: Test
            ---
            # Content

            Body text"#},
        FrontmatterSplit::Complete {
            yaml: "title: Test",
            body: indoc! {r#"
                # Content

                Body text"#},
        }
    )]
    #[case::no_frontmatter(
        indoc! {r#"
            # Content

            Body text"#},
        FrontmatterSplit::NoFrontmatter
    )]
    #[case::empty_body(
        indoc! {r#"
            ---
            title: Test
            ---
        "#},
        FrontmatterSplit::Complete { yaml: "title: Test", body: "" }
    )]
    #[case::whitespace_handling(
        indoc! {r#"
               ---
            title: Test
            ---

            # Content"#},
        FrontmatterSplit::Complete {
            yaml: "title: Test",
            body: indoc! {r#"

                # Content"#},
        }
    )]
    #[case::multiple_frontmatter_separators(
        indoc! {r#"
            ---
            title: Test
            ---
            # Section
            ---
            More content"#},
        FrontmatterSplit::Complete {
            yaml: "title: Test",
            body: indoc! {r#"
                # Section
                ---
                More content"#},
        }
    )]
    #[case::frontmatter_with_complex_yaml(
        indoc! {r#"
            ---
            title: "Complex: Title"
            tags: ["tag1", "tag2"]
            ---
            ## Heading"#},
        FrontmatterSplit::Complete {
            yaml: indoc! {r#"
                title: "Complex: Title"
                tags: ["tag1", "tag2"]"#},
            body: "## Heading"
        }
    )]
    fn test_split_frontmatter(#[case] input: &str, #[case] expected: FrontmatterSplit<'_>) {
        let result = split_frontmatter(input).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_split_frontmatter_rejects_unterminated_frontmatter() {
        let content = indoc! {r#"
            ---
            title: Test
            # Content

            Body text
        "#};

        let result = split_frontmatter(content);

        assert!(matches!(
            result,
            Err(IngestError::Parse(message)) if message.contains("unterminated front‑matter")
        ));
    }

    #[rstest]
    #[case::body_with_trailing_newline(
        indoc! {r#"
            ---
            title: Test
            is_completed: true
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            ---
            # Content
        "#},
        indoc! {r#"
            # Content
        "#}
    )]
    #[case::body_with_blank_line(
        indoc! {r#"
            ---
            title: Test
            is_completed: true
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            ---
            # Content

            Body text"#},
        indoc! {r#"
            # Content

            Body text"#}
    )]
    fn test_parse_obsidian_file_preserves_markdown_body(
        #[case] content: &str,
        #[case] expected_body: &str,
    ) -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("preserve.md");
        fs::write(&file_path, content)?;

        let parsed_file = parse_obsidian_file(&file_path)?.unwrap();

        assert_eq!(parsed_file.markdown_body, expected_body);
        Ok(())
    }
}
