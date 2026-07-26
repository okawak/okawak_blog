use crate::classify::ParsedArticleFile;
use regex::Regex;
use std::{collections::HashMap, sync::LazyLock};

/// Published article hrefs indexed by extensionless source keys.
pub(crate) struct Index {
    routes: HashMap<String, String>,
}

impl Index {
    pub(crate) fn from_articles(articles: &[ParsedArticleFile]) -> Self {
        let routes = articles
            .iter()
            .map(|article| {
                (
                    article.source_key.clone(),
                    format!("/{}/{}", article.category.as_str(), article.slug),
                )
            })
            .collect();
        Self { routes }
    }

    fn resolve(&self, target: &str) -> Option<&str> {
        self.routes.get(target).map(String::as_str).or_else(|| {
            let suffix = format!("/{target}");
            self.routes.iter().find_map(|(source_key, href)| {
                source_key.ends_with(&suffix).then_some(href.as_str())
            })
        })
    }
}

/// Convert Obsidian internal links to published Markdown links.
pub(crate) fn convert(content: &str, index: &Index) -> String {
    static OBSIDIAN_LINK_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]\]").expect("Invalid regex pattern"));

    OBSIDIAN_LINK_REGEX
        .replace_all(content, |captures: &regex::Captures| {
            let link_content = &captures[1];

            let (link_target, display_text) = if let Some(pipe_position) = link_content.find('|') {
                let (link, display) = link_content.split_at(pipe_position);
                (link.trim(), display[1..].trim())
            } else {
                (link_content.trim(), link_content.trim())
            };

            let href = index
                .resolve(link_target)
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    log::warn!("Internal link target '{link_target}' was not found");
                    format!("/{link_target}")
                });

            format!(
                "[{}]({})",
                escape_markdown_link_text(display_text),
                escape_markdown_link_destination(&href)
            )
        })
        .to_string()
}

fn escape_markdown_link_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_markdown_link_destination(destination: &str) -> String {
    destination.replace(')', "\\)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{ContentKind, ObsidianFrontMatter, convert_markdown_to_html};
    use domain::{Category, SectionPath, Slug};

    fn index(routes: &[(&str, &str)]) -> Index {
        Index {
            routes: routes
                .iter()
                .map(|(source_key, href)| ((*source_key).to_string(), (*href).to_string()))
                .collect(),
        }
    }

    fn article(source_key: &str, category: Category, slug: &str) -> ParsedArticleFile {
        ParsedArticleFile {
            category,
            slug: Slug::new(slug.to_string()).unwrap(),
            source_key: source_key.to_string(),
            section_path: SectionPath::default(),
            markdown_body: "# Article".to_string(),
            front_matter: ObsidianFrontMatter {
                title: "Article".to_string(),
                kind: ContentKind::Article,
                tags: None,
                summary: None,
                priority: None,
                created: "2025-01-01T00:00:00+09:00".to_string(),
                updated: "2025-01-01T00:00:00+09:00".to_string(),
                is_completed: true,
                category: Some(category.as_str().to_string()),
                page: None,
            },
        }
    }

    #[test]
    fn index_is_built_from_articles() {
        let articles = vec![article("sub/dir/test", Category::Tech, "slug")];

        let index = Index::from_articles(&articles);

        assert_eq!(index.resolve("sub/dir/test"), Some("/tech/slug"));
    }

    #[test]
    fn index_is_empty_when_there_are_no_articles() {
        let index = Index::from_articles(&[]);

        assert_eq!(index.resolve("test"), None);
    }

    #[test]
    fn index_preserves_distinct_source_keys() {
        let articles = vec![
            article("dir1/test", Category::Tech, "slug1"),
            article("dir2/test", Category::Daily, "slug2"),
        ];

        let index = Index::from_articles(&articles);

        assert_eq!(index.resolve("dir1/test"), Some("/tech/slug1"));
        assert_eq!(index.resolve("dir2/test"), Some("/daily/slug2"));
    }

    #[test]
    fn index_resolves_exact_and_path_suffix_targets() {
        let index = index(&[
            ("notes/another-note", "/tech/abc123def"),
            ("filename", "/daily/xyz789abc"),
        ]);

        assert_eq!(index.resolve("notes/another-note"), Some("/tech/abc123def"));
        assert_eq!(index.resolve("another-note"), Some("/tech/abc123def"));
        assert_eq!(index.resolve("filename"), Some("/daily/xyz789abc"));
        assert_eq!(index.resolve("missing"), None);
    }

    #[test]
    fn convert_internal_links() {
        let index = index(&[
            ("notes/another-note", "/tech/abc123def"),
            ("filename", "/daily/xyz789abc"),
        ]);

        assert_eq!(
            convert("Check out [[another-note]] for more info.", &index),
            "Check out [another-note](/tech/abc123def) for more info."
        );
        assert_eq!(
            convert("See [[filename|Custom Display Text]] here.", &index),
            "See [Custom Display Text](/daily/xyz789abc) here."
        );
        assert_eq!(
            convert("Link to [[nonexistent]] file.", &index),
            "Link to [nonexistent](/nonexistent) file."
        );
        assert_eq!(
            convert("This is normal text with no special links.", &index),
            "This is normal text with no special links."
        );
    }

    #[test]
    fn convert_escapes_markdown_link_parts() {
        let index = index(&[("File with <script>", "/tech/abc123")]);

        assert_eq!(
            convert("[[File with <script>|Display & test]]", &index),
            "[Display & test](/tech/abc123)"
        );
        assert_eq!(
            convert("[[File \"quoted\"|Text with 'quotes']]", &index),
            "[Text with 'quotes'](/File \"quoted\")"
        );
    }

    #[test]
    fn converted_links_are_rendered_as_html() {
        let markdown = r#"# My Article

This is a test with [[Another Article|link]] and **bold** text.

## Section Two

- Item with [[Reference Note]]
- Regular item"#;
        let index = index(&[
            ("Another Article", "/tech/def456"),
            ("Reference Note", "/daily/ghi789"),
        ]);

        let markdown = convert(markdown, &index);
        let html = convert_markdown_to_html(&markdown).unwrap();

        assert!(html.contains("<h1>My Article</h1>"));
        assert!(html.contains("<a href=\"/tech/def456\">link</a>"));
        assert!(html.contains("<a href=\"/daily/ghi789\">Reference Note</a>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<ul>"));
    }
}
