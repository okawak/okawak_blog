use crate::config::Config;
use crate::error::{ObsidianError, Result};
use crate::types::{ParsedArticleFile, ParsedCategoryFile, ParsedPageFile};
use domain::{Category, ContentKind, PageKey};
use ingest::{FileMapping, ObsidianFrontMatter, ParsedObsidianFile, parse_obsidian_file};
use log::{error, warn};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// ファイル分類の結果を保持する構造体。
pub(crate) struct ClassifiedFiles {
    pub(crate) articles: Vec<ParsedArticleFile>,
    pub(crate) pages: Vec<ParsedPageFile>,
    pub(crate) categories: Vec<ParsedCategoryFile>,
    pub(crate) skipped: usize,
    pub(crate) errors: usize,
}

/// Obsidian ファイルのリストを解析し、記事・ページ・カテゴリに分類する。
///  でないファイルはスキップ、解析エラーはエラーとしてカウントする。
pub(crate) fn classify_obsidian_files(
    markdown_files: Vec<PathBuf>,
    config: &Config,
) -> ClassifiedFiles {
    let mut articles = Vec::new();
    let mut pages = Vec::new();
    let mut categories = Vec::new();
    let mut skipped = 0usize;
    let mut errors = 0usize;

    for file_path in markdown_files {
        match parse_obsidian_file(&file_path) {
            Ok(Some(parsed)) if parsed.front_matter.is_completed => {
                let result: Result<()> = match parsed.front_matter.kind {
                    ContentKind::Article => process_valid_article_file(&file_path, parsed, config)
                        .map(|f| articles.push(f)),
                    ContentKind::Page => {
                        process_valid_page_file(parsed).map(|f| pages.push(f))
                    }
                    ContentKind::Home => {
                        process_valid_home_file(parsed).map(|f| pages.push(f))
                    }
                    ContentKind::Category => {
                        process_valid_category_file(parsed).map(|f| categories.push(f))
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

    ClassifiedFiles { articles, pages, categories, skipped, errors }
}

fn process_valid_article_file(
    file_path: &Path,
    parsed_file: ParsedObsidianFile,
    config: &Config,
) -> Result<ParsedArticleFile> {
    let relative_path = get_relative_path(file_path, &config.obsidian_dir)?;
    let slug = crate::slug::generate_slug(
        &parsed_file.front_matter.title,
        relative_path,
        &parsed_file.front_matter.created,
    )?;
    let category = parse_category(&parsed_file.front_matter)?;
    let mapping_key = normalize_path_for_url(&relative_path.with_extension(""));
    let section_path =
        derive_section_path(relative_path, parsed_file.front_matter.category.as_deref());

    Ok(ParsedArticleFile {
        category,
        slug,
        mapping_key,
        section_path,
        markdown_body: parsed_file.markdown_body,
        front_matter: parsed_file.front_matter,
    })
}

fn process_valid_page_file(parsed_file: ParsedObsidianFile) -> Result<ParsedPageFile> {
    Ok(ParsedPageFile {
        page: parse_page_key(&parsed_file.front_matter)?,
        markdown_body: parsed_file.markdown_body,
        front_matter: parsed_file.front_matter,
    })
}

fn process_valid_home_file(parsed_file: ParsedObsidianFile) -> Result<ParsedPageFile> {
    Ok(ParsedPageFile {
        page: PageKey::new("home".to_string())
            .map_err(|error| ObsidianError::Parse(error.to_string()))?,
        markdown_body: parsed_file.markdown_body,
        front_matter: parsed_file.front_matter,
    })
}

fn process_valid_category_file(parsed_file: ParsedObsidianFile) -> Result<ParsedCategoryFile> {
    Ok(ParsedCategoryFile {
        category: parse_category(&parsed_file.front_matter)?,
        markdown_body: parsed_file.markdown_body,
        front_matter: parsed_file.front_matter,
    })
}

/// OS 固有のパス区切り文字を  に統一する。
fn normalize_path_for_url(path: &Path) -> String {
    path.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/")
}

/// ベースディレクトリを除いた相対パスを取得する。
fn get_relative_path<'a>(file_path: &'a Path, base_dir: &Path) -> Result<&'a Path> {
    file_path.strip_prefix(base_dir).map_err(|_| {
        ObsidianError::Path(format!(
            "Failed to strip prefix from {}",
            file_path.display()
        ))
    })
}

pub(crate) fn build_file_mapping(valid_files: &[ParsedArticleFile]) -> FileMapping {
    let mut mapping = FileMapping::with_capacity(valid_files.len());
    for parsed_file in valid_files {
        mapping.insert(
            parsed_file.mapping_key.clone(),
            format!("/{}/{}", parsed_file.category.as_str(), parsed_file.slug),
        );
    }
    mapping
}

fn derive_section_path(relative_path: &Path, category: Option<&str>) -> Vec<String> {
    let mut path_components: Vec<String> = relative_path
        .parent()
        .map(|parent| {
            parent
                .iter()
                .map(|component| component.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();

    if let (Some(category), Some(first_component)) = (category, path_components.first())
        && first_component == category
    {
        path_components.remove(0);
    }

    path_components
}

pub(crate) fn ensure_unique_page_keys(valid_pages: &[ParsedPageFile]) -> Result<()> {
    let mut seen = HashSet::with_capacity(valid_pages.len());
    for parsed_page in valid_pages {
        if !seen.insert(parsed_page.page.as_str()) {
            return Err(ObsidianError::Parse(format!(
                "Duplicate page key detected: {}",
                parsed_page.page.as_str()
            )));
        }
    }
    Ok(())
}

pub(crate) fn ensure_unique_category_landings(
    valid_categories: &[ParsedCategoryFile],
) -> Result<()> {
    let mut seen = HashSet::with_capacity(valid_categories.len());
    for parsed_category in valid_categories {
        if !seen.insert(parsed_category.category.as_str()) {
            return Err(ObsidianError::Parse(format!(
                "Duplicate category landing detected: {}",
                parsed_category.category.as_str()
            )));
        }
    }
    Ok(())
}

pub(crate) fn parse_category(front_matter: &ObsidianFrontMatter) -> Result<Category> {
    let category = front_matter
        .category
        .as_deref()
        .ok_or_else(|| ObsidianError::Parse("Completed articles require a category".to_string()))?;
    category.parse().map_err(Into::into)
}

pub(crate) fn parse_page_key(front_matter: &ObsidianFrontMatter) -> Result<PageKey> {
    let page = front_matter
        .page
        .as_deref()
        .ok_or_else(|| ObsidianError::Parse("Completed pages require a page key".to_string()))?;
    PageKey::new(page.trim().to_string()).map_err(|error| ObsidianError::Parse(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_build_file_mapping_success() {
        let front_matter = ObsidianFrontMatter {
            title: "Test Article".to_string(),
            kind: ContentKind::Article,
            tags: Some(vec!["test".to_string()]),
            summary: Some("Test summary".to_string()),
            priority: Some(1),
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-02T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("tech".to_string()),
            page: None,
        };

        let parsed_file = ParsedArticleFile {
            category: Category::Tech,
            slug: "slug".to_string(),
            mapping_key: "test".to_string(),
            section_path: vec![],
            markdown_body: "# Test Content".to_string(),
            front_matter,
        };
        let valid_files = vec![parsed_file];
        let mapping = build_file_mapping(&valid_files);

        assert_eq!(mapping.len(), 1);
        assert!(mapping.contains_key("test"));
        assert_eq!(mapping.get("test").unwrap(), "/tech/slug");
    }

    #[rstest]
    fn test_build_file_mapping_empty() {
        let valid_files: Vec<ParsedArticleFile> = vec![];
        let mapping = build_file_mapping(&valid_files);

        assert_eq!(mapping.len(), 0);
    }

    #[rstest]
    fn test_build_file_mapping_path_collision() {
        let front_matter1 = ObsidianFrontMatter {
            title: "Test Article 1".to_string(),
            kind: ContentKind::Article,
            tags: Some(vec!["test1".to_string()]),
            summary: Some("Test summary 1".to_string()),
            priority: Some(1),
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-02T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("tech".to_string()),
            page: None,
        };

        let front_matter2 = ObsidianFrontMatter {
            title: "Test Article 2".to_string(),
            kind: ContentKind::Article,
            tags: Some(vec!["test2".to_string()]),
            summary: Some("Test summary 2".to_string()),
            priority: Some(2),
            created: "2025-01-03T00:00:00+09:00".to_string(),
            updated: "2025-01-04T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("daily".to_string()),
            page: None,
        };

        let parsed_file1 = ParsedArticleFile {
            category: Category::Tech,
            slug: "slug1".to_string(),
            mapping_key: "dir1/test".to_string(),
            section_path: vec!["dir1".to_string()],
            markdown_body: "# Test Content 1".to_string(),
            front_matter: front_matter1,
        };
        let parsed_file2 = ParsedArticleFile {
            category: Category::Daily,
            slug: "slug2".to_string(),
            mapping_key: "dir2/test".to_string(),
            section_path: vec!["dir2".to_string()],
            markdown_body: "# Test Content 2".to_string(),
            front_matter: front_matter2,
        };
        let valid_files = vec![parsed_file1, parsed_file2];
        let mapping = build_file_mapping(&valid_files);

        assert_eq!(mapping.len(), 2);
        assert!(mapping.contains_key("dir1/test"));
        assert!(mapping.contains_key("dir2/test"));
    }

    #[rstest]
    fn test_build_file_mapping_url_normalization() {
        let front_matter = ObsidianFrontMatter {
            title: "URL Test".to_string(),
            kind: ContentKind::Article,
            tags: None,
            summary: None,
            priority: None,
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-01T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("tech".to_string()),
            page: None,
        };

        let parsed_file = ParsedArticleFile {
            category: Category::Tech,
            slug: "slug".to_string(),
            mapping_key: "sub/dir/test".to_string(),
            section_path: vec!["sub".to_string(), "dir".to_string()],
            markdown_body: "# URL Test Content".to_string(),
            front_matter,
        };
        let valid_files = vec![parsed_file];
        let mapping = build_file_mapping(&valid_files);

        let href = mapping.get("sub/dir/test").unwrap();
        assert_eq!(href, "/tech/slug");
    }

    #[test]
    fn test_ensure_unique_category_landings_rejects_duplicates() {
        let valid_categories = vec![
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

        let result = ensure_unique_category_landings(&valid_categories);

        assert!(
            matches!(result, Err(ObsidianError::Parse(message)) if message.contains("Duplicate category landing"))
        );
    }

    #[test]
    fn test_derive_section_path_drops_category_root() {
        let section_path = derive_section_path(Path::new("tech/block1/hoge.md"), Some("tech"));

        assert_eq!(section_path, vec!["block1".to_string()]);
    }

    #[test]
    fn test_derive_section_path_keeps_nested_sections() {
        let section_path = derive_section_path(Path::new("tech/rust/async/hoge.md"), Some("tech"));

        assert_eq!(section_path, vec!["rust".to_string(), "async".to_string()]);
    }

    #[test]
    fn test_parse_page_key_success() {
        let front_matter = ObsidianFrontMatter {
            title: "About".to_string(),
            kind: ContentKind::Page,
            tags: None,
            summary: Some("About this site".to_string()),
            priority: None,
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-01T00:00:00+09:00".to_string(),
            is_completed: true,
            category: None,
            page: Some("about".to_string()),
        };

        assert_eq!(parse_page_key(&front_matter).unwrap().as_str(), "about");
    }

    #[test]
    fn test_parse_page_key_rejects_nested_path() {
        let front_matter = ObsidianFrontMatter {
            title: "About".to_string(),
            kind: ContentKind::Page,
            tags: None,
            summary: None,
            priority: None,
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-01T00:00:00+09:00".to_string(),
            is_completed: true,
            category: None,
            page: Some("about/team".to_string()),
        };

        assert!(matches!(
            parse_page_key(&front_matter),
            Err(ObsidianError::Parse(_))
        ));
    }

    #[test]
    fn test_parse_page_key_rejects_uppercase() {
        let front_matter = ObsidianFrontMatter {
            title: "About".to_string(),
            kind: ContentKind::Page,
            tags: None,
            summary: None,
            priority: None,
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-01T00:00:00+09:00".to_string(),
            is_completed: true,
            category: None,
            page: Some("About".to_string()),
        };

        assert!(matches!(
            parse_page_key(&front_matter),
            Err(ObsidianError::Parse(_))
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
            Err(ObsidianError::Parse(message)) if message.contains("Duplicate page key")
        ));
    }
}
