use crate::error::{PublishError, Result};
use crate::vault::{ContentKind, ObsidianFrontMatter, ParsedObsidianFile, parse_obsidian_file};
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
    /// Extensionless vault-relative key used to resolve Obsidian internal links.
    pub(crate) source_key: String,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedHomeFile {
    /// Extensionless vault-relative key used to resolve Obsidian internal links.
    pub(crate) source_key: String,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedCategoryFile {
    pub(crate) category: Category,
    /// Extensionless vault-relative key used to resolve Obsidian internal links.
    pub(crate) source_key: String,
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
                let result =
                    derive_source_key(&file_path, obsidian_dir).and_then(|source_key| match parsed
                        .front_matter
                        .kind
                    {
                        ContentKind::Article => {
                            process_article_file(&file_path, parsed, obsidian_dir, source_key)
                                .map(|file| articles.push(file))
                        }
                        ContentKind::Page => parse_page_key(parsed.front_matter.page.as_deref())
                            .map(|page| {
                                pages.push(ParsedPageFile {
                                    page,
                                    source_key,
                                    markdown_body: parsed.markdown_body,
                                    front_matter: parsed.front_matter,
                                });
                            }),
                        ContentKind::Home => {
                            if home.is_some() {
                                Err(PublishError::Parse(
                                    "Duplicate home content detected".to_string(),
                                ))
                            } else {
                                home = Some(ParsedHomeFile {
                                    source_key,
                                    markdown_body: parsed.markdown_body,
                                    front_matter: parsed.front_matter,
                                });
                                Ok(())
                            }
                        }
                        ContentKind::Category => parse_category(
                            parsed.front_matter.category.as_deref(),
                        )
                        .map(|category| {
                            categories.push(ParsedCategoryFile {
                                category,
                                source_key,
                                markdown_body: parsed.markdown_body,
                                front_matter: parsed.front_matter,
                            });
                        }),
                    });
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
    source_key: String,
) -> Result<ParsedArticleFile> {
    let relative_path = file_path.strip_prefix(obsidian_dir)?;
    let category = parse_category(parsed_file.front_matter.category.as_deref())?;
    let category_relative_path = relative_path.strip_prefix(category.as_str())?;
    let slug = crate::slug::generate_slug(
        &parsed_file.front_matter.title,
        relative_path,
        &parsed_file.front_matter.created,
    )?;
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

fn derive_source_key(file_path: &Path, obsidian_dir: &Path) -> Result<String> {
    Ok(file_path
        .strip_prefix(obsidian_dir)?
        .with_extension("")
        .to_string_lossy()
        .into_owned())
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
    PageKey::new(page.trim().to_string()).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    const TEST_TIMESTAMP: &str = "2025-01-01T00:00:00+09:00";

    fn front_matter(kind: ContentKind) -> ObsidianFrontMatter {
        ObsidianFrontMatter {
            title: "Test".to_string(),
            kind,
            tags: None,
            summary: None,
            is_completed: true,
            priority: None,
            created: TEST_TIMESTAMP.to_string(),
            updated: TEST_TIMESTAMP.to_string(),
            category: None,
            page: None,
        }
    }

    fn parsed_article(category: Category) -> ParsedObsidianFile {
        let mut front_matter = front_matter(ContentKind::Article);
        front_matter.category = Some(category.as_str().to_string());
        ParsedObsidianFile {
            front_matter,
            markdown_body: String::new(),
        }
    }

    fn parsed_category(category: Category) -> ParsedCategoryFile {
        let mut front_matter = front_matter(ContentKind::Category);
        front_matter.category = Some(category.as_str().to_string());
        ParsedCategoryFile {
            category,
            source_key: format!("{}/index", category.as_str()),
            markdown_body: String::new(),
            front_matter,
        }
    }

    fn parsed_page(page: &str) -> ParsedPageFile {
        let mut front_matter = front_matter(ContentKind::Page);
        front_matter.page = Some(page.to_string());
        ParsedPageFile {
            page: PageKey::new(page.to_string()).unwrap(),
            source_key: format!("pages/{page}"),
            markdown_body: String::new(),
            front_matter,
        }
    }

    #[test]
    fn test_ensure_unique_category_landings_rejects_duplicates() {
        let categories = vec![
            parsed_category(Category::Tech),
            parsed_category(Category::Tech),
        ];

        let result = ensure_unique_category_landings(&categories);

        assert!(
            matches!(result, Err(PublishError::Parse(message)) if message.contains("Duplicate category landing"))
        );
    }

    #[test]
    fn test_process_article_file_rejects_category_path_mismatch() {
        let obsidian_dir = Path::new("/vault");
        let parsed_file = parsed_article(Category::Tech);

        let result = process_article_file(
            Path::new("/vault/daily/article.md"),
            parsed_file,
            obsidian_dir,
            "daily/article".to_string(),
        );

        assert!(matches!(result, Err(PublishError::StripPrefix(_))));
    }

    #[test]
    fn test_derive_source_key_uses_extensionless_vault_relative_path() {
        let source_key =
            derive_source_key(Path::new("/vault/pages/about.md"), Path::new("/vault")).unwrap();

        assert_eq!(source_key, "pages/about");
    }

    #[rstest]
    #[case::single_section("block1/hoge.md", &["block1"])]
    #[case::nested_sections("rust/async/hoge.md", &["rust", "async"])]
    #[case::root_article("hoge.md", &[])]
    fn test_derive_section_path(#[case] path: &str, #[case] expected: &[&str]) {
        let section_path = derive_section_path(Path::new(path));

        assert_eq!(section_path.segments(), expected);
    }

    #[test]
    fn test_parse_page_key_success() {
        assert_eq!(parse_page_key(Some(" about ")).unwrap().as_str(), "about");
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
        let parsed_pages = vec![parsed_page("about"), parsed_page("about")];

        assert!(matches!(
            ensure_unique_page_keys(&parsed_pages),
            Err(PublishError::Parse(message)) if message.contains("Duplicate page key")
        ));
    }
}
