use crate::classify::ClassifiedFiles;
use pulldown_cmark::{CowStr, Event, LinkType, Tag};
use std::collections::HashMap;

const ROUTED_PAGE_KEYS: &[&str] = &["about"];

/// Published content hrefs indexed by extensionless source keys.
#[derive(Default)]
pub(crate) struct Index {
    routes: HashMap<String, String>,
}

impl Index {
    pub(crate) fn from_classified_files(files: &ClassifiedFiles) -> Self {
        let article_routes = files.articles.iter().map(|article| {
            (
                article.source_key.clone(),
                format!("/{}/{}", article.category.as_str(), article.slug),
            )
        });
        let page_routes = files
            .pages
            .iter()
            .filter(|page| ROUTED_PAGE_KEYS.contains(&page.page.as_str()))
            .map(|page| (page.source_key.clone(), format!("/{}", page.page.as_str())));
        let home_routes = files
            .home
            .iter()
            .map(|home| (home.source_key.clone(), "/".to_string()));
        let category_routes = files.categories.iter().map(|category| {
            (
                category.source_key.clone(),
                format!("/{}", category.category.as_str()),
            )
        });

        let routes = article_routes
            .chain(page_routes)
            .chain(home_routes)
            .chain(category_routes)
            .collect();

        Self { routes }
    }

    /// Resolve an exact vault-relative key or an Obsidian-style filename reference.
    fn resolve(&self, target: &str) -> Option<&str> {
        self.routes.get(target).map(String::as_str).or_else(|| {
            let suffix = format!("/{target}");
            self.routes.iter().find_map(|(source_key, href)| {
                source_key.ends_with(&suffix).then_some(href.as_str())
            })
        })
    }
}

/// Resolve Obsidian WikiLink events to published URLs.
pub(crate) fn resolve_wikilinks<'a>(
    events: impl Iterator<Item = Event<'a>> + 'a,
    index: &'a Index,
) -> impl Iterator<Item = Event<'a>> + 'a {
    events.map(move |event| match event {
        Event::Start(Tag::Link {
            link_type: link_type @ LinkType::WikiLink { has_pothole },
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: resolve_wikilink_destination(&dest_url, has_pothole, index),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type: link_type @ LinkType::WikiLink { has_pothole },
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: resolve_wikilink_destination(&dest_url, has_pothole, index),
            title,
            id,
        }),
        event => event,
    })
}

fn resolve_wikilink_destination<'a>(
    target: &str,
    has_pothole: bool,
    index: &'a Index,
) -> CowStr<'a> {
    let target = target.trim();
    // pulldown-cmark keeps the escape before a piped WikiLink delimiter in its target.
    let target = if has_pothole {
        target.strip_suffix('\\').unwrap_or(target)
    } else {
        target
    };

    match index.resolve(target) {
        Some(href) => href.into(),
        None => {
            tracing::warn!(%target, "internal link target was not found");
            format!("/{target}").into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{
        ClassifiedFiles, ParsedArticleFile, ParsedCategoryFile, ParsedHomeFile, ParsedPageFile,
    };
    use crate::vault::{ContentKind, ObsidianFrontMatter};
    use domain::{Category, SectionPath, Slug};
    use pulldown_cmark::{Options, Parser};
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

    fn resolved_destinations(markdown: &str, index: &Index) -> Vec<(&'static str, String)> {
        let parser = Parser::new_ext(markdown, Options::ENABLE_WIKILINKS);
        resolve_wikilinks(parser, index)
            .filter_map(|event| match event {
                Event::Start(Tag::Link {
                    link_type: LinkType::WikiLink { .. },
                    dest_url,
                    ..
                }) => Some(("link", dest_url.to_string())),
                Event::Start(Tag::Image {
                    link_type: LinkType::WikiLink { .. },
                    dest_url,
                    ..
                }) => Some(("image", dest_url.to_string())),
                _ => None,
            })
            .collect()
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
    fn resolve_wikilinks_to_published_urls() {
        let index = index(&[
            ("notes/another-note", "/tech/abc123def"),
            ("filename", "/daily/xyz789abc"),
        ]);

        assert_eq!(
            resolved_destinations(
                "[[another-note]] [[filename|Label]] ![[filename]] ![[filename|Alt]] [[missing]]",
                &index,
            ),
            vec![
                ("link", "/tech/abc123def".to_string()),
                ("link", "/daily/xyz789abc".to_string()),
                ("image", "/daily/xyz789abc".to_string()),
                ("image", "/daily/xyz789abc".to_string()),
                ("link", "/missing".to_string()),
            ]
        );
    }

    #[test]
    fn resolve_wikilinks_accepts_table_escaped_pipe_delimiters() {
        let index = index(&[("article", "/tech/slug")]);

        assert_eq!(
            resolved_destinations(r"[[article\|Label]] ![[article\|Alt]]", &index),
            vec![
                ("link", "/tech/slug".to_string()),
                ("image", "/tech/slug".to_string()),
            ]
        );
    }

    #[rstest]
    #[case::inline_code("`[[ignored]]` and [[article]]")]
    #[case::fenced_code_block("```markdown\n[[ignored]]\n```\n\n[[article]]")]
    #[case::tilde_fenced_code_block("~~~markdown\n[[ignored]]\n~~~\n\n[[article]]")]
    #[case::indented_code_block("    [[ignored]]\n\n[[article]]")]
    fn resolve_wikilinks_ignores_syntax_in_code(#[case] markdown: &str) {
        let index = index(&[("article", "/tech/slug")]);

        assert_eq!(
            resolved_destinations(markdown, &index),
            vec![("link", "/tech/slug".to_string())]
        );
    }
}
