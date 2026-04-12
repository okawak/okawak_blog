pub mod config;
pub mod error;
pub mod slug;

/// Async fn that takes page HTML and returns enriched HTML with rich bookmark cards.
/// Use `offline_bookmark_enricher` in tests to avoid network access.
pub type BookmarkEnricher =
    Arc<dyn Fn(String) -> BoxFuture<'static, bookmark::Result<String>> + Send + Sync>;

/// Returns a `BookmarkEnricher` that uses fallback data only; no network requests.
pub fn offline_bookmark_enricher() -> BookmarkEnricher {
    Arc::new(|html: String| {
        Box::pin(async move { bookmark::convert_simple_bookmarks_to_rich_offline(&html).await })
    })
}

use artifacts::{
    CategoryLandingMetadata, SiteDirectories, build_site_artifacts, write_article_page,
    write_category_page, write_page_document, write_site_artifacts,
};
pub use config::Config;
use domain::{
    ArticleBody, ArticleMeta, ArticleMetaInput, Category, ContentKind, PageArtifactDocument,
    PageKey, Slug, Title,
};
pub use error::{ObsidianError, Result};
use ingest::{
    FileMapping, ObsidianFrontMatter, ParsedObsidianFile, convert_markdown_to_html,
    convert_obsidian_links, parse_obsidian_file, scan_obsidian_files,
};

use futures::{StreamExt, TryStreamExt, future::BoxFuture, stream};
use log::{error, info, warn};
use std::{collections::HashSet, path::Path, sync::Arc};

struct RenderedArticle {
    meta: ArticleMeta,
    html: String,
}

struct RenderedPage {
    document: PageArtifactDocument,
}

struct RenderedCategoryLanding {
    metadata: CategoryLandingMetadata,
    html: String,
}

struct ParsedArticleFile {
    slug: String,
    mapping_key: String,
    section_path: Vec<String>,
    markdown_body: String,
    front_matter: ObsidianFrontMatter,
}

struct ParsedPageFile {
    page: PageKey,
    markdown_body: String,
    front_matter: ObsidianFrontMatter,
}

struct ParsedCategoryFile {
    category: Category,
    markdown_body: String,
    front_matter: ObsidianFrontMatter,
}

pub async fn run_main(config: &Config) -> Result<()> {
    let enrich: BookmarkEnricher = Arc::new(|html: String| {
        Box::pin(async move { bookmark::convert_simple_bookmarks_to_rich(&html).await })
    });
    run_with_enricher(config, enrich).await
}

pub async fn run_with_enricher(config: &Config, enrich: BookmarkEnricher) -> Result<()> {
    let start_time = std::time::Instant::now();
    info!("=== Publisher Started ===");
    info!("Input directory: {}", config.obsidian_dir.display());
    info!("Output directory: {}", config.output_dir.display());

    let markdown_files = scan_obsidian_files(&config.obsidian_dir)?;
    info!("Found {} markdown files", markdown_files.len());

    let mut skipped_count = 0;
    let mut error_count = 0;

    let mut valid_articles = Vec::new();
    let mut valid_pages = Vec::new();
    let mut valid_categories = Vec::new();
    for file_path in markdown_files {
        match parse_obsidian_file(&file_path) {
            Ok(Some(parsed_file)) if parsed_file.front_matter.is_completed => {
                match parsed_file.front_matter.kind {
                    ContentKind::Article => {
                        match process_valid_article_file(&file_path, parsed_file, config) {
                            Ok(parsed_file) => valid_articles.push(parsed_file),
                            Err(e) => {
                                error_count += 1;
                                error!("Error processing {}: {}", file_path.display(), e);
                            }
                        }
                    }
                    ContentKind::Page => match process_valid_page_file(parsed_file) {
                        Ok(parsed_file) => valid_pages.push(parsed_file),
                        Err(e) => {
                            error_count += 1;
                            error!("Error processing {}: {}", file_path.display(), e);
                        }
                    },
                    ContentKind::Category => match process_valid_category_file(parsed_file) {
                        Ok(parsed_file) => valid_categories.push(parsed_file),
                        Err(e) => {
                            error_count += 1;
                            error!("Error processing {}: {}", file_path.display(), e);
                        }
                    },
                    kind => {
                        skipped_count += 1;
                        info!(
                            "Skipped (kind not implemented yet): {} [{}]",
                            file_path.display(),
                            kind
                        );
                    }
                }
            }
            Ok(_) => {
                skipped_count += 1;
                warn!("Skipped (not completed): {}", file_path.display());
            }
            Err(e) => {
                error_count += 1;
                error!("Error processing {}: {}", file_path.display(), e);
            }
        }
    }

    info!("Valid article files: {}", valid_articles.len());
    info!("Valid page files: {}", valid_pages.len());
    info!("Valid category files: {}", valid_categories.len());
    info!("Skipped files: {skipped_count}");
    if error_count > 0 {
        warn!("Error files: {error_count}");
    }

    ensure_unique_page_keys(&valid_pages)?;
    ensure_unique_category_landings(&valid_categories)?;

    let file_mapping = build_file_mapping(&valid_articles);
    let site_directories = SiteDirectories::prepare(&config.output_dir)?;

    const CONCURRENT_LIMIT: usize = 4;
    let article_metas: Vec<ArticleMeta> = stream::iter(valid_articles)
        .map(|parsed_file| {
            let enrich = Arc::clone(&enrich);
            render_publishable_article(parsed_file, &file_mapping, enrich)
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .try_fold(Vec::new(), |mut article_metas, rendered_article| {
            let site_directories = site_directories.clone();
            async move {
                let (rendered_article, output_file_path) = tokio::task::spawn_blocking(move || {
                    let output_file_path = write_article_page(
                        &site_directories,
                        &rendered_article.meta.slug,
                        &rendered_article.html,
                    )?;
                    Ok::<_, artifacts::ArtifactsError>((rendered_article, output_file_path))
                })
                .await??;
                info!("...processed {}", output_file_path.display());
                article_metas.push(rendered_article.meta);
                Ok(article_metas)
            }
        })
        .await?;

    let rendered_pages: Vec<RenderedPage> = stream::iter(valid_pages)
        .map(|parsed_file| {
            let enrich = Arc::clone(&enrich);
            render_static_page(parsed_file, &file_mapping, enrich)
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .try_collect()
        .await?;

    let rendered_categories: Vec<RenderedCategoryLanding> = stream::iter(valid_categories)
        .map(|parsed_file| {
            let enrich = Arc::clone(&enrich);
            render_category_landing(parsed_file, &file_mapping, enrich)
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .try_collect()
        .await?;

    for rendered_page in rendered_pages {
        let site_directories = site_directories.clone();
        let output_file_path = tokio::task::spawn_blocking(move || {
            write_page_document(&site_directories, &rendered_page.document)
        })
        .await??;
        info!("...processed {}", output_file_path.display());
    }

    let mut category_landings = Vec::with_capacity(rendered_categories.len());
    for rendered_category in rendered_categories {
        let site_directories = site_directories.clone();
        let (metadata, output_file_path) = tokio::task::spawn_blocking(move || {
            let output_file_path = write_category_page(
                &site_directories,
                rendered_category.metadata.category,
                &rendered_category.html,
            )?;
            Ok::<_, artifacts::ArtifactsError>((rendered_category.metadata, output_file_path))
        })
        .await??;
        info!("...processed {}", output_file_path.display());
        category_landings.push(metadata);
    }

    let site_artifacts = build_site_artifacts(article_metas, category_landings);
    let site_directories_for_write = site_directories.clone();
    let site_artifacts = tokio::task::spawn_blocking(move || {
        write_site_artifacts(&site_directories_for_write, &site_artifacts)?;
        Ok::<_, artifacts::ArtifactsError>(site_artifacts)
    })
    .await??;

    let processed_count = site_artifacts.article_index.len();
    let duration = start_time.elapsed();

    // Print the processing summary.
    info!("=== Processing Summary ===");
    info!("Successfully processed: {processed_count} files");
    info!("  Skipped: {skipped_count} files");
    if error_count > 0 {
        warn!("  Errors: {error_count} files");
    }
    info!("  Processing time: {duration:.2?}");
    info!("Output directory: {}", config.output_dir.display());

    // Print details for the processed files.
    if !site_artifacts.article_index.is_empty() {
        info!("Processed files:");
        for article in &site_artifacts.article_index {
            info!("  • {} ({})", article.title.as_str(), article.slug.as_str());
        }
    }

    info!("=== Publisher Completed ===");
    Ok(())
}

/// Processes a valid file and builds a `ParsedFile`.
fn process_valid_article_file(
    file_path: &Path,
    parsed_file: ParsedObsidianFile,
    config: &Config,
) -> Result<ParsedArticleFile> {
    let relative_path = get_relative_path(file_path, &config.obsidian_dir)?;
    let slug = slug::generate_slug(
        &parsed_file.front_matter.title,
        relative_path,
        &parsed_file.front_matter.created,
    )?;
    let mapping_key = normalize_path_for_url(&relative_path.with_extension(""));
    let section_path =
        derive_section_path(relative_path, parsed_file.front_matter.category.as_deref());

    Ok(ParsedArticleFile {
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

fn process_valid_category_file(parsed_file: ParsedObsidianFile) -> Result<ParsedCategoryFile> {
    Ok(ParsedCategoryFile {
        category: parse_category(&parsed_file.front_matter)?,
        markdown_body: parsed_file.markdown_body,
        front_matter: parsed_file.front_matter,
    })
}

/// Normalizes a path for use in URLs by converting OS-specific separators to `/`.
fn normalize_path_for_url(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    path_str.replace(std::path::MAIN_SEPARATOR, "/")
}

/// Shared helper for extracting a relative path.
fn get_relative_path<'a>(file_path: &'a Path, base_dir: &Path) -> Result<&'a Path> {
    file_path.strip_prefix(base_dir).map_err(|_| {
        ObsidianError::Path(format!(
            "Failed to strip prefix from {}",
            file_path.display()
        ))
    })
}

fn build_file_mapping(valid_files: &[ParsedArticleFile]) -> FileMapping {
    let mut mapping = FileMapping::with_capacity(valid_files.len());

    for parsed_file in valid_files {
        mapping.insert(parsed_file.mapping_key.clone(), parsed_file.slug.clone());
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

fn ensure_unique_page_keys(valid_pages: &[ParsedPageFile]) -> Result<()> {
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

fn ensure_unique_category_landings(valid_categories: &[ParsedCategoryFile]) -> Result<()> {
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

async fn process_parsed_file(
    parsed_file: ParsedArticleFile,
    file_mapping: &FileMapping,
    enrich: &BookmarkEnricher,
) -> Result<RenderedArticle> {
    let markdown_with_links = convert_obsidian_links(&parsed_file.markdown_body, file_mapping);
    let html_body = convert_markdown_to_html(&markdown_with_links)?;

    // Convert simple bookmark markup into rich bookmark cards after rendering HTML.
    let fallback = html_body.clone();
    let html_with_rich_bookmarks = enrich(html_body).await.unwrap_or_else(|e| {
        warn!("Warning: Failed to convert simple bookmarks to rich bookmarks: {e}");
        fallback
    });

    let category = parse_category(&parsed_file.front_matter)?;
    let meta = ArticleMeta::new(ArticleMetaInput {
        slug: Slug::new(parsed_file.slug)?,
        title: Title::new(parsed_file.front_matter.title)?,
        category,
        section_path: parsed_file.section_path,
        description: parsed_file.front_matter.summary,
        tags: parsed_file.front_matter.tags.unwrap_or_default(),
        priority: parsed_file.front_matter.priority,
        created_at: parsed_file.front_matter.created,
        updated_at: parsed_file.front_matter.updated,
    })?;
    let body = ArticleBody::new(html_with_rich_bookmarks)?;

    Ok(RenderedArticle {
        meta,
        html: body.html,
    })
}

async fn render_publishable_article(
    parsed_file: ParsedArticleFile,
    file_mapping: &FileMapping,
    enrich: BookmarkEnricher,
) -> Result<RenderedArticle> {
    process_parsed_file(parsed_file, file_mapping, &enrich).await
}

async fn process_page_file(
    parsed_file: ParsedPageFile,
    file_mapping: &FileMapping,
    enrich: &BookmarkEnricher,
) -> Result<RenderedPage> {
    let markdown_with_links = convert_obsidian_links(&parsed_file.markdown_body, file_mapping);
    let html_body = convert_markdown_to_html(&markdown_with_links)?;

    let fallback = html_body.clone();
    let html_with_rich_bookmarks = enrich(html_body).await.unwrap_or_else(|e| {
        warn!("Warning: Failed to convert simple bookmarks to rich bookmarks: {e}");
        fallback
    });

    Ok(RenderedPage {
        document: PageArtifactDocument {
            page: parsed_file.page,
            title: parsed_file.front_matter.title,
            description: parsed_file.front_matter.summary,
            html: html_with_rich_bookmarks,
            updated_at: parsed_file.front_matter.updated,
        },
    })
}

async fn render_static_page(
    parsed_file: ParsedPageFile,
    file_mapping: &FileMapping,
    enrich: BookmarkEnricher,
) -> Result<RenderedPage> {
    process_page_file(parsed_file, file_mapping, &enrich).await
}

async fn process_category_file(
    parsed_file: ParsedCategoryFile,
    file_mapping: &FileMapping,
    enrich: &BookmarkEnricher,
) -> Result<RenderedCategoryLanding> {
    let markdown_with_links = convert_obsidian_links(&parsed_file.markdown_body, file_mapping);
    let html_body = convert_markdown_to_html(&markdown_with_links)?;

    let fallback = html_body.clone();
    let html_with_rich_bookmarks = enrich(html_body).await.unwrap_or_else(|e| {
        warn!("Warning: Failed to convert simple bookmarks to rich bookmarks: {e}");
        fallback
    });

    Ok(RenderedCategoryLanding {
        metadata: CategoryLandingMetadata {
            category: parsed_file.category,
            title: parsed_file.front_matter.title,
            description: parsed_file.front_matter.summary,
            updated_at: parsed_file.front_matter.updated,
        },
        html: html_with_rich_bookmarks,
    })
}

async fn render_category_landing(
    parsed_file: ParsedCategoryFile,
    file_mapping: &FileMapping,
    enrich: BookmarkEnricher,
) -> Result<RenderedCategoryLanding> {
    process_category_file(parsed_file, file_mapping, &enrich).await
}

fn parse_category(front_matter: &ObsidianFrontMatter) -> Result<Category> {
    let category = front_matter
        .category
        .as_deref()
        .ok_or_else(|| ObsidianError::Parse("Completed articles require a category".to_string()))?;

    category.parse().map_err(Into::into)
}

fn parse_page_key(front_matter: &ObsidianFrontMatter) -> Result<PageKey> {
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
            category: Some("blog".to_string()),
            page: None,
        };

        let parsed_file1 = ParsedArticleFile {
            slug: "slug1".to_string(),
            mapping_key: "dir1/test".to_string(),
            section_path: vec!["dir1".to_string()],
            markdown_body: "# Test Content 1".to_string(),
            front_matter: front_matter1,
        };
        let parsed_file2 = ParsedArticleFile {
            slug: "slug2".to_string(),
            mapping_key: "dir2/test".to_string(),
            section_path: vec!["dir2".to_string()],
            markdown_body: "# Test Content 2".to_string(),
            front_matter: front_matter2,
        };
        let valid_files = vec![parsed_file1, parsed_file2];
        let mapping = build_file_mapping(&valid_files);

        // Using the full relative path as the key prevents collisions.
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
            category: None,
            page: None,
        };

        let parsed_file = ParsedArticleFile {
            slug: "slug".to_string(),
            mapping_key: "sub/dir/test".to_string(),
            section_path: vec!["sub".to_string(), "dir".to_string()],
            markdown_body: "# URL Test Content".to_string(),
            front_matter,
        };
        let valid_files = vec![parsed_file];
        let mapping = build_file_mapping(&valid_files);

        let slug = mapping.get("sub/dir/test").unwrap();
        assert!(!slug.is_empty());
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
