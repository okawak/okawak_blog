use crate::classify::ClassifiedFiles;
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};
use std::collections::HashMap;

const ROUTED_PAGE_KEYS: &[&str] = &["about"];

/// Published content hrefs indexed by extensionless source keys.
#[derive(Default)]
pub(crate) struct Index {
    routes: HashMap<String, String>,
}

impl Index {
    pub(crate) fn from_classified_files(files: &ClassifiedFiles) -> Self {
        let routed_page_count = files
            .pages
            .iter()
            .filter(|page| is_routed_page(page.page.as_str()))
            .count();
        let capacity = files.articles.len()
            + routed_page_count
            + usize::from(files.home.is_some())
            + files.categories.len();
        let mut routes = HashMap::with_capacity(capacity);

        routes.extend(files.articles.iter().map(|article| {
            (
                article.source_key.clone(),
                format!("/{}/{}", article.category.as_str(), article.slug),
            )
        }));
        routes.extend(
            files
                .pages
                .iter()
                .filter(|page| is_routed_page(page.page.as_str()))
                .map(|page| (page.source_key.clone(), format!("/{}", page.page.as_str()))),
        );
        routes.extend(
            files
                .home
                .iter()
                .map(|home| (home.source_key.clone(), "/".to_string())),
        );
        routes.extend(files.categories.iter().map(|category| {
            (
                category.source_key.clone(),
                format!("/{}", category.category.as_str()),
            )
        }));

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

fn is_routed_page(page_key: &str) -> bool {
    ROUTED_PAGE_KEYS.contains(&page_key)
}

/// Resolve Obsidian internal links to published Markdown links outside code.
/// The vault is expected to contain valid Obsidian syntax; malformed links remain unchanged.
pub(crate) fn resolve_internal_links(content: &str, index: &Index) -> String {
    let mut resolved = String::with_capacity(content.len());
    let mut copied_until = 0;

    for (event, range) in Parser::new_ext(content, Options::ENABLE_WIKILINKS).into_offset_iter() {
        if !matches!(
            event,
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                ..
            })
        ) {
            continue;
        }

        resolved.push_str(&content[copied_until..range.start]);
        resolved.push_str(&resolve_internal_link(&content[range.clone()], index));
        copied_until = range.end;
    }

    resolved.push_str(&content[copied_until..]);
    resolved
}

fn resolve_internal_link(link: &str, index: &Index) -> String {
    let link_content = link
        .strip_prefix("[[")
        .and_then(|link| link.strip_suffix("]]"))
        .expect("pulldown-cmark should return the complete internal link range");
    let (link_target, display_text) = link_content.split_once('|').map_or(
        (link_content.trim(), link_content.trim()),
        |(target, display)| (target.trim(), display.trim()),
    );
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
    use crate::classify::{
        ClassifiedFiles, ParsedArticleFile, ParsedCategoryFile, ParsedHomeFile, ParsedPageFile,
    };
    use crate::render::convert_markdown_to_html;
    use crate::vault::{ContentKind, ObsidianFrontMatter};
    use domain::{Category, SectionPath, Slug};
    use rstest::rstest;

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

    fn page(source_key: &str, page: &str) -> ParsedPageFile {
        ParsedPageFile {
            page: domain::PageKey::new(page.to_string()).unwrap(),
            source_key: source_key.to_string(),
            markdown_body: "# Page".to_string(),
            front_matter: front_matter(ContentKind::Page),
        }
    }

    fn home(source_key: &str) -> ParsedHomeFile {
        ParsedHomeFile {
            source_key: source_key.to_string(),
            markdown_body: "# Home".to_string(),
            front_matter: front_matter(ContentKind::Home),
        }
    }

    fn category(source_key: &str, category: Category) -> ParsedCategoryFile {
        ParsedCategoryFile {
            category,
            source_key: source_key.to_string(),
            markdown_body: "# Category".to_string(),
            front_matter: front_matter(ContentKind::Category),
        }
    }

    fn front_matter(kind: ContentKind) -> ObsidianFrontMatter {
        ObsidianFrontMatter {
            title: "Content".to_string(),
            kind,
            tags: None,
            summary: None,
            priority: None,
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-01T00:00:00+09:00".to_string(),
            is_completed: true,
            category: None,
            page: None,
        }
    }

    fn classified_files(articles: Vec<ParsedArticleFile>) -> ClassifiedFiles {
        ClassifiedFiles {
            articles,
            pages: Vec::new(),
            home: None,
            categories: Vec::new(),
            skipped: 0,
            errors: 0,
        }
    }

    #[test]
    fn index_is_built_from_all_published_content() {
        let files = ClassifiedFiles {
            articles: vec![article("tech/article", Category::Tech, "slug")],
            pages: vec![page("pages/about", "about")],
            home: Some(home("home")),
            categories: vec![category("tech/index", Category::Tech)],
            skipped: 0,
            errors: 0,
        };

        let index = Index::from_classified_files(&files);

        assert_eq!(index.resolve("tech/article"), Some("/tech/slug"));
        assert_eq!(index.resolve("pages/about"), Some("/about"));
        assert_eq!(index.resolve("home"), Some("/"));
        assert_eq!(index.resolve("tech/index"), Some("/tech"));
    }

    #[test]
    fn index_excludes_pages_without_a_published_route() {
        let files = ClassifiedFiles {
            articles: Vec::new(),
            pages: vec![page("pages/contact", "contact")],
            home: None,
            categories: Vec::new(),
            skipped: 0,
            errors: 0,
        };

        let index = Index::from_classified_files(&files);

        assert_eq!(index.resolve("pages/contact"), None);
        assert_eq!(index.resolve("contact"), None);
    }

    #[test]
    fn index_is_empty_when_there_is_no_published_content() {
        let index = Index::from_classified_files(&classified_files(Vec::new()));

        assert_eq!(index.resolve("test"), None);
    }

    #[test]
    fn index_preserves_distinct_source_keys() {
        let files = classified_files(vec![
            article("dir1/test", Category::Tech, "slug1"),
            article("dir2/test", Category::Daily, "slug2"),
        ]);

        let index = Index::from_classified_files(&files);

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
    fn resolve_internal_links_to_markdown() {
        let index = index(&[
            ("notes/another-note", "/tech/abc123def"),
            ("filename", "/daily/xyz789abc"),
        ]);

        assert_eq!(
            resolve_internal_links("Check out [[another-note]] for more info.", &index),
            "Check out [another-note](/tech/abc123def) for more info."
        );
        assert_eq!(
            resolve_internal_links("See [[filename|Custom Display Text]] here.", &index),
            "See [Custom Display Text](/daily/xyz789abc) here."
        );
        assert_eq!(
            resolve_internal_links("Link to [[nonexistent]] file.", &index),
            "Link to [nonexistent](/nonexistent) file."
        );
        assert_eq!(
            resolve_internal_links("This is normal text with no special links.", &index),
            "This is normal text with no special links."
        );
    }

    #[test]
    fn resolve_internal_links_escapes_markdown_link_parts() {
        let index = index(&[("File with <script>", "/tech/abc123")]);

        assert_eq!(
            resolve_internal_links("[[File with <script>|Display & test]]", &index),
            "[Display & test](/tech/abc123)"
        );
        assert_eq!(
            resolve_internal_links("[[File \"quoted\"|Text with 'quotes']]", &index),
            "[Text with 'quotes'](/File \"quoted\")"
        );
    }

    #[rstest]
    #[case::inline_code(
        "`[[article]]` and [[article]]",
        "`[[article]]` and [article](/tech/slug)"
    )]
    #[case::fenced_code_block(
        "```markdown\n[[article]]\n```\n\n[[article]]",
        "```markdown\n[[article]]\n```\n\n[article](/tech/slug)"
    )]
    #[case::tilde_fenced_code_block(
        "~~~markdown\n[[article]]\n~~~\n\n[[article]]",
        "~~~markdown\n[[article]]\n~~~\n\n[article](/tech/slug)"
    )]
    #[case::indented_code_block(
        "    [[article]]\n\n[[article]]",
        "    [[article]]\n\n[article](/tech/slug)"
    )]
    fn resolve_internal_links_preserves_syntax_in_code(
        #[case] markdown: &str,
        #[case] expected: &str,
    ) {
        let index = index(&[("article", "/tech/slug")]);

        assert_eq!(resolve_internal_links(markdown, &index), expected);
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

        let markdown = resolve_internal_links(markdown, &index);
        let html = convert_markdown_to_html(&markdown).unwrap();

        assert!(html.contains("<h1>My Article</h1>"));
        assert!(html.contains("<a href=\"/tech/def456\">link</a>"));
        assert!(html.contains("<a href=\"/daily/ghi789\">Reference Note</a>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<ul>"));
    }
}
