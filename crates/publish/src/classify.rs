use crate::error::{PublishError, Result};
use crate::ingest::{ContentKind, ObsidianFrontMatter, ParsedObsidianFile, parse_obsidian_file};
use domain::{Category, PageKey, SectionPath, Slug};
use log::{error, warn};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) struct ParsedArticleFile {
    pub(crate) category: Category,
    pub(crate) slug: Slug,
    /// Extensionless vault-relative key used to resolve Obsidian internal links.
    pub(crate) source_key: String,
    /// Category-relative directories used to group articles in category navigation.
    pub(crate) section_path: SectionPath,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedPageFile {
    pub(crate) page: PageKey,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedHomeFile {
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedCategoryFile {
    pub(crate) category: Category,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ClassifiedFiles {
    pub(crate) articles: Vec<ParsedArticleFile>,
    pub(crate) pages: Vec<ParsedPageFile>,
    pub(crate) home: Option<ParsedHomeFile>,
    pub(crate) categories: Vec<ParsedCategoryFile>,
    pub(crate) skipped: usize,
    pub(crate) errors: usize,
}

pub(crate) fn classify_obsidian_files(
    markdown_files: Vec<PathBuf>,
    obsidian_dir: &Path,
) -> ClassifiedFiles {
    let mut articles = Vec::new();
    let mut pages = Vec::new();
    let mut home = None;
    let mut categories = Vec::new();
    let mut skipped = 0usize;
    let mut errors = 0usize;

    for file_path in markdown_files {
        match parse_obsidian_file(&file_path) {
            Ok(Some(parsed)) if parsed.front_matter.is_completed => {
                let result: Result<()> = match parsed.front_matter.kind {
                    ContentKind::Article => process_article_file(&file_path, parsed, obsidian_dir)
                        .map(|f| articles.push(f)),
                    ContentKind::Page => {
                        parse_page_key(parsed.front_matter.page.as_deref()).map(|page| {
                            pages.push(ParsedPageFile {
                                page,
                                markdown_body: parsed.markdown_body,
                                front_matter: parsed.front_matter,
                            });
                        })
                    }
                    ContentKind::Home => {
                        if home.is_some() {
                            Err(PublishError::Parse(
                                "Duplicate home content detected".to_string(),
                            ))
                        } else {
                            home = Some(ParsedHomeFile {
                                markdown_body: parsed.markdown_body,
                                front_matter: parsed.front_matter,
                            });
                            Ok(())
                        }
                    }
                    ContentKind::Category => {
                        parse_category(parsed.front_matter.category.as_deref()).map(|category| {
                            categories.push(ParsedCategoryFile {
                                category,
                                markdown_body: parsed.markdown_body,
                                front_matter: parsed.front_matter,
                            });
                        })
                    }
                };
                if let Err(e) = result {
                    errors += 1;
                    error!("Error processing {}: {}", file_path.display(), e);
                }
            }
            Ok(_) => {
                skipped += 1;
                warn!("Skipped (not completed): {}", file_path.display());
            }
            Err(e) => {
                errors += 1;
                error!("Error processing {}: {}", file_path.display(), e);
            }
        }
    }

    ClassifiedFiles {
        articles,
        pages,
        home,
        categories,
        skipped,
        errors,
    }
}

fn process_article_file(
    file_path: &Path,
    parsed_file: ParsedObsidianFile,
    obsidian_dir: &Path,
) -> Result<ParsedArticleFile> {
    let relative_path = file_path.strip_prefix(obsidian_dir)?;
    let category = parse_category(parsed_file.front_matter.category.as_deref())?;
    let category_relative_path = relative_path.strip_prefix(category.as_str())?;
    let slug = crate::slug::generate_slug(
        &parsed_file.front_matter.title,
        relative_path,
        &parsed_file.front_matter.created,
    )?;
    let source_key = relative_path
        .with_extension("")
        .to_string_lossy()
        .into_owned();
    let section_path = derive_section_path(category_relative_path);

    Ok(ParsedArticleFile {
        category,
        slug,
        source_key,
        section_path,
        markdown_body: parsed_file.markdown_body,
        front_matter: parsed_file.front_matter,
    })
}

fn derive_section_path(category_relative_path: &Path) -> SectionPath {
    let segments = category_relative_path
        .parent()
        .map(|parent| {
            parent
                .iter()
                .map(|component| component.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    SectionPath::new(segments)
}

pub(crate) fn ensure_unique_page_keys(pages: &[ParsedPageFile]) -> Result<()> {
    let mut seen = HashSet::with_capacity(pages.len());
    for parsed_page in pages {
        if !seen.insert(parsed_page.page.as_str()) {
            return Err(PublishError::Parse(format!(
                "Duplicate page key detected: {}",
                parsed_page.page.as_str()
            )));
        }
    }
    Ok(())
}

pub(crate) fn ensure_unique_category_landings(categories: &[ParsedCategoryFile]) -> Result<()> {
    let mut seen = HashSet::with_capacity(categories.len());
    for parsed_category in categories {
        if !seen.insert(parsed_category.category.as_str()) {
            return Err(PublishError::Parse(format!(
                "Duplicate category landing detected: {}",
                parsed_category.category.as_str()
            )));
        }
    }
    Ok(())
}

fn parse_category(category: Option<&str>) -> Result<Category> {
    let category = category
        .ok_or_else(|| PublishError::Parse("Completed content requires a category".to_string()))?;
    category.parse().map_err(Into::into)
}

fn parse_page_key(page: Option<&str>) -> Result<PageKey> {
    let page =
        page.ok_or_else(|| PublishError::Parse("Completed pages require a page key".to_string()))?;
    let page = page.trim();
    if page == "home" {
        return Err(PublishError::Parse(
            "Static pages cannot use the reserved home page key".to_string(),
        ));
    }
    PageKey::new(page.to_string()).map_err(|error| PublishError::Parse(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_unique_category_landings_rejects_duplicates() {
        let categories = vec![
            ParsedCategoryFile {
                category: Category::Tech,
                markdown_body: "# Tech".to_string(),
                front_matter: ObsidianFrontMatter {
                    title: "Tech".to_string(),
                    kind: ContentKind::Category,
                    tags: None,
                    summary: None,
                    is_completed: true,
                    priority: None,
                    created: "2025-01-01T00:00:00+09:00".to_string(),
                    updated: "2025-01-01T00:00:00+09:00".to_string(),
                    category: Some("tech".to_string()),
                    page: None,
                },
            },
            ParsedCategoryFile {
                category: Category::Tech,
                markdown_body: "# Tech again".to_string(),
                front_matter: ObsidianFrontMatter {
                    title: "Tech Again".to_string(),
                    kind: ContentKind::Category,
                    tags: None,
                    summary: None,
                    is_completed: true,
                    priority: None,
                    created: "2025-01-01T00:00:00+09:00".to_string(),
                    updated: "2025-01-01T00:00:00+09:00".to_string(),
                    category: Some("tech".to_string()),
                    page: None,
                },
            },
        ];

        let result = ensure_unique_category_landings(&categories);

        assert!(
            matches!(result, Err(PublishError::Parse(message)) if message.contains("Duplicate category landing"))
        );
    }

    #[test]
    fn test_process_article_file_rejects_category_path_mismatch() {
        let obsidian_dir = Path::new("/vault");
        let parsed_file = ParsedObsidianFile {
            front_matter: ObsidianFrontMatter {
                title: "Test Article".to_string(),
                kind: ContentKind::Article,
                tags: None,
                summary: None,
                priority: None,
                created: "2025-01-01T00:00:00+09:00".to_string(),
                updated: "2025-01-01T00:00:00+09:00".to_string(),
                is_completed: true,
                category: Some("tech".to_string()),
                page: None,
            },
            markdown_body: "# Test Article".to_string(),
        };

        let result = process_article_file(
            Path::new("/vault/daily/article.md"),
            parsed_file,
            obsidian_dir,
        );

        assert!(matches!(result, Err(PublishError::StripPrefix(_))));
    }

    #[test]
    fn test_derive_section_path_from_category_relative_article() {
        let section_path = derive_section_path(Path::new("block1/hoge.md"));

        assert_eq!(section_path.segments(), ["block1"]);
    }

    #[test]
    fn test_derive_section_path_keeps_nested_sections() {
        let section_path = derive_section_path(Path::new("rust/async/hoge.md"));

        assert_eq!(section_path.segments(), ["rust", "async"]);
    }

    #[test]
    fn test_parse_page_key_success() {
        assert_eq!(parse_page_key(Some("about")).unwrap().as_str(), "about");
    }

    #[test]
    fn test_parse_page_key_rejects_nested_path() {
        assert!(matches!(
            parse_page_key(Some("about/team")),
            Err(PublishError::Parse(_))
        ));
    }

    #[test]
    fn test_parse_page_key_rejects_reserved_home_key() {
        assert!(matches!(
            parse_page_key(Some("home")),
            Err(PublishError::Parse(_))
        ));
    }

    #[test]
    fn test_parse_page_key_rejects_uppercase() {
        assert!(matches!(
            parse_page_key(Some("About")),
            Err(PublishError::Parse(_))
        ));
    }

    #[test]
    fn test_parse_page_key_rejects_missing_value() {
        assert!(matches!(
            parse_page_key(None),
            Err(PublishError::Parse(message)) if message.contains("require a page key")
        ));
    }

    #[test]
    fn test_parse_category_success() {
        assert_eq!(parse_category(Some("tech")).unwrap(), Category::Tech);
    }

    #[test]
    fn test_parse_category_rejects_missing_value() {
        assert!(matches!(
            parse_category(None),
            Err(PublishError::Parse(message)) if message.contains("requires a category")
        ));
    }

    #[test]
    fn test_ensure_unique_page_keys_rejects_duplicates() {
        let parsed_pages = vec![
            ParsedPageFile {
                page: PageKey::new("about".to_string()).unwrap(),
                markdown_body: "# About".to_string(),
                front_matter: ObsidianFrontMatter {
                    title: "About".to_string(),
                    kind: ContentKind::Page,
                    tags: None,
                    summary: None,
                    priority: None,
                    created: "2025-01-01T00:00:00+09:00".to_string(),
                    updated: "2025-01-01T00:00:00+09:00".to_string(),
                    is_completed: true,
                    category: None,
                    page: Some("about".to_string()),
                },
            },
            ParsedPageFile {
                page: PageKey::new("about".to_string()).unwrap(),
                markdown_body: "# About 2".to_string(),
                front_matter: ObsidianFrontMatter {
                    title: "About 2".to_string(),
                    kind: ContentKind::Page,
                    tags: None,
                    summary: None,
                    priority: None,
                    created: "2025-01-01T00:00:00+09:00".to_string(),
                    updated: "2025-01-01T00:00:00+09:00".to_string(),
                    is_completed: true,
                    category: None,
                    page: Some("about".to_string()),
                },
            },
        ];

        assert!(matches!(
            ensure_unique_page_keys(&parsed_pages),
            Err(PublishError::Parse(message)) if message.contains("Duplicate page key")
        ));
    }
}
