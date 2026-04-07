pub mod config;
pub mod error;
pub mod slug;

use artifacts::{SiteDirectories, build_site_artifacts, write_article_page, write_site_artifacts};
use bookmark::convert_simple_bookmarks_to_rich;
pub use config::Config;
use domain::{ArticleBody, ArticleMeta, ArticleMetaInput, Category, Slug, Title};
pub use error::{ObsidianError, Result};
pub use ingest::ObsidianFrontMatter;
use ingest::{
    FileMapping, ParsedObsidianFile, convert_markdown_to_html, convert_obsidian_links,
    parse_obsidian_file, scan_obsidian_files,
};

use futures::{StreamExt, TryStreamExt, stream};
use log::{error, info, warn};
use std::path::{Path, PathBuf};

struct RenderedArticle {
    meta: ArticleMeta,
    html: String,
}

/// Holds the parsed file data used by the publisher flow.
struct ParsedFile {
    file_path: PathBuf,
    slug: String,
    markdown_body: String,
    front_matter: ObsidianFrontMatter,
}

pub async fn run_main(config: &Config) -> Result<()> {
    let start_time = std::time::Instant::now();
    info!("=== Publisher Started ===");
    info!("Input directory: {}", config.obsidian_dir.display());
    info!("Output directory: {}", config.output_dir.display());

    let markdown_files = scan_obsidian_files(&config.obsidian_dir)?;
    info!("Found {} markdown files", markdown_files.len());

    let mut skipped_count = 0;
    let mut error_count = 0;

    let valid_files: Vec<ParsedFile> = markdown_files
        .into_iter()
        .filter_map(|file_path| match parse_obsidian_file(&file_path) {
            Ok(Some(parsed_file)) if parsed_file.front_matter.is_completed => {
                match process_valid_file(&file_path, parsed_file, config) {
                    Ok(parsed_file) => Some(parsed_file),
                    Err(e) => {
                        error_count += 1;
                        error!("Error processing {}: {}", file_path.display(), e);
                        None
                    }
                }
            }
            Ok(_) => {
                skipped_count += 1;
                warn!("Skipped (not completed): {}", file_path.display());
                None
            }
            Err(e) => {
                error_count += 1;
                error!("Error processing {}: {}", file_path.display(), e);
                None
            }
        })
        .collect();

    info!("Valid files: {}", valid_files.len());
    info!("Skipped files: {skipped_count}");
    if error_count > 0 {
        warn!("Error files: {error_count}");
    }

    let file_mapping = build_file_mapping(config, &valid_files)?;
    let site_directories = SiteDirectories::prepare(&config.output_dir)?;

    const CONCURRENT_LIMIT: usize = 4;
    let article_metas: Vec<ArticleMeta> = stream::iter(valid_files)
        .map(|parsed_file| render_publishable_article(parsed_file, &file_mapping))
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
    let site_artifacts = build_site_artifacts(article_metas);
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
fn process_valid_file(
    file_path: &Path,
    parsed_file: ParsedObsidianFile,
    config: &Config,
) -> Result<ParsedFile> {
    let relative_path = get_relative_path(file_path, &config.obsidian_dir)?;
    let slug = slug::generate_slug(
        &parsed_file.front_matter.title,
        relative_path,
        &parsed_file.front_matter.created,
    )?;

    Ok(ParsedFile {
        file_path: file_path.to_path_buf(),
        slug,
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

fn build_file_mapping(config: &Config, valid_files: &[ParsedFile]) -> Result<FileMapping> {
    let mut mapping = FileMapping::with_capacity(valid_files.len());

    for parsed_file in valid_files {
        let relative_path = get_relative_path(&parsed_file.file_path, &config.obsidian_dir)?;
        let relative_path_no_ext = relative_path.with_extension("");
        let mapping_key = normalize_path_for_url(&relative_path_no_ext);
        mapping.insert(mapping_key, parsed_file.slug.clone());
    }

    Ok(mapping)
}

async fn process_parsed_file(
    parsed_file: ParsedFile,
    file_mapping: &FileMapping,
) -> Result<RenderedArticle> {
    let markdown_with_links = convert_obsidian_links(&parsed_file.markdown_body, file_mapping);
    let html_body = convert_markdown_to_html(&markdown_with_links)?;

    // Convert simple bookmark markup into rich bookmark cards after rendering HTML.
    let html_with_rich_bookmarks = convert_simple_bookmarks_to_rich(&html_body)
        .await
        .unwrap_or_else(|e| {
            warn!("Warning: Failed to convert simple bookmarks to rich bookmarks: {e}");
            html_body
        });

    let category = parse_category(&parsed_file.front_matter)?;
    let meta = ArticleMeta::new(ArticleMetaInput {
        slug: Slug::new(parsed_file.slug)?,
        title: Title::new(parsed_file.front_matter.title)?,
        category,
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
    parsed_file: ParsedFile,
    file_mapping: &FileMapping,
) -> Result<RenderedArticle> {
    process_parsed_file(parsed_file, file_mapping).await
}

fn parse_category(front_matter: &ObsidianFrontMatter) -> Result<Category> {
    let category = front_matter
        .category
        .as_deref()
        .ok_or_else(|| ObsidianError::Parse("Completed articles require a category".to_string()))?;

    category.parse().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[rstest]
    fn test_build_file_mapping_success() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            obsidian_dir: temp_dir.path().to_path_buf(),
            output_dir: PathBuf::from("output"),
        };

        let file_path = temp_dir.path().join("test.md");
        let front_matter = ObsidianFrontMatter {
            title: "Test Article".to_string(),
            tags: Some(vec!["test".to_string()]),
            summary: Some("Test summary".to_string()),
            priority: Some(1),
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-02T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("tech".to_string()),
        };

        let parsed_file = ParsedFile {
            file_path,
            slug: "slug".to_string(),
            markdown_body: "# Test Content".to_string(),
            front_matter,
        };
        let valid_files = vec![parsed_file];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        assert_eq!(mapping.len(), 1);
        assert!(mapping.contains_key("test")); // ファイル名がキーとなる
    }

    #[rstest]
    fn test_build_file_mapping_empty() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            obsidian_dir: temp_dir.path().to_path_buf(),
            output_dir: PathBuf::from("output"),
        };

        let valid_files: Vec<ParsedFile> = vec![];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        assert_eq!(mapping.len(), 0);
    }

    #[rstest]
    fn test_build_file_mapping_path_collision() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            obsidian_dir: temp_dir.path().to_path_buf(),
            output_dir: PathBuf::from("output"),
        };

        // Files with the same name in different directories.
        let file_path1 = temp_dir.path().join("dir1").join("test.md");
        let file_path2 = temp_dir.path().join("dir2").join("test.md");

        let front_matter1 = ObsidianFrontMatter {
            title: "Test Article 1".to_string(),
            tags: Some(vec!["test1".to_string()]),
            summary: Some("Test summary 1".to_string()),
            priority: Some(1),
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-02T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("tech".to_string()),
        };

        let front_matter2 = ObsidianFrontMatter {
            title: "Test Article 2".to_string(),
            tags: Some(vec!["test2".to_string()]),
            summary: Some("Test summary 2".to_string()),
            priority: Some(2),
            created: "2025-01-03T00:00:00+09:00".to_string(),
            updated: "2025-01-04T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("blog".to_string()),
        };

        let parsed_file1 = ParsedFile {
            file_path: file_path1,
            slug: "slug1".to_string(),
            markdown_body: "# Test Content 1".to_string(),
            front_matter: front_matter1,
        };
        let parsed_file2 = ParsedFile {
            file_path: file_path2,
            slug: "slug2".to_string(),
            markdown_body: "# Test Content 2".to_string(),
            front_matter: front_matter2,
        };
        let valid_files = vec![parsed_file1, parsed_file2];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        // Using the full relative path as the key prevents collisions.
        assert_eq!(mapping.len(), 2);
        assert!(mapping.contains_key("dir1/test"));
        assert!(mapping.contains_key("dir2/test"));
    }

    #[rstest]
    fn test_build_file_mapping_url_normalization() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            obsidian_dir: temp_dir.path().to_path_buf(),
            output_dir: PathBuf::from("output"),
        };

        let file_path = temp_dir.path().join("sub").join("dir").join("test.md");
        let front_matter = ObsidianFrontMatter {
            title: "URL Test".to_string(),
            tags: None,
            summary: None,
            priority: None,
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-01T00:00:00+09:00".to_string(),
            is_completed: true,
            category: None,
        };

        let parsed_file = ParsedFile {
            file_path,
            slug: "slug".to_string(),
            markdown_body: "# URL Test Content".to_string(),
            front_matter,
        };
        let valid_files = vec![parsed_file];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        let slug = mapping.get("sub/dir/test").unwrap();

        // Verify URL normalization uses Unix-style separators.
        // Ensure the slug is present.
        assert!(!slug.is_empty());
    }
}
