mod artifacts;
mod bookmark;
mod classify;
pub mod error;
mod ingest;
mod render;
pub mod slug;
mod types;

use crate::artifacts::{
    SiteDirectories, build_site_artifacts, validate_site_artifacts, write_article_page,
    write_category_page, write_page_document, write_site_artifacts,
};
pub use crate::bookmark::BookmarkError;
use crate::classify::{
    build_file_mapping, classify_obsidian_files, ensure_unique_category_landings,
    ensure_unique_page_keys,
};
use crate::ingest::scan_obsidian_files;
use crate::render::{render_article, render_category, render_page};
pub use error::{ObsidianError, Result};
use futures::{StreamExt, TryStreamExt, future::BoxFuture, stream};
use log::info;
use std::{path::Path, sync::Arc};

/// Async function that enriches page HTML with rich bookmark cards.
pub type BookmarkEnricher = Arc<
    dyn Fn(String) -> BoxFuture<'static, std::result::Result<String, BookmarkError>> + Send + Sync,
>;

fn rich_bookmark_enricher() -> BookmarkEnricher {
    Arc::new(|html: String| {
        Box::pin(async move { crate::bookmark::convert_simple_bookmarks_to_rich(&html).await })
    })
}

pub async fn publish(obsidian_dir: &Path, output_dir: &Path) -> Result<()> {
    publish_with_bookmark_enricher(obsidian_dir, output_dir, rich_bookmark_enricher()).await
}

pub async fn publish_with_bookmark_enricher(
    obsidian_dir: &Path,
    output_dir: &Path,
    enrich: BookmarkEnricher,
) -> Result<()> {
    validate_obsidian_dir(obsidian_dir)?;

    let start_time = std::time::Instant::now();
    info!("=== Publisher Started ===");
    info!("Input directory: {}", obsidian_dir.display());
    info!("Output directory: {}", output_dir.display());

    let markdown_files = scan_obsidian_files(obsidian_dir)?;
    info!("Found {} markdown files", markdown_files.len());

    let classify::ClassifiedFiles {
        articles,
        pages,
        categories,
        skipped,
        errors,
    } = classify_obsidian_files(markdown_files, obsidian_dir);

    info!("Valid article files: {}", articles.len());
    info!("Valid page files: {}", pages.len());
    info!("Valid category files: {}", categories.len());
    info!("Skipped files: {skipped}");
    if errors > 0 {
        return Err(ObsidianError::ContentErrors { count: errors });
    }

    ensure_unique_page_keys(&pages)?;
    ensure_unique_category_landings(&categories)?;

    let file_mapping = build_file_mapping(&articles);
    let site_directories = SiteDirectories::prepare(output_dir)?;

    const CONCURRENT_LIMIT: usize = 4;

    let article_metas = stream::iter(articles)
        .map(|parsed_file| {
            let enrich = Arc::clone(&enrich);
            render_article(parsed_file, &file_mapping, enrich)
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .try_fold(Vec::new(), |mut article_metas, rendered| {
            let site_directories = site_directories.clone();
            async move {
                let (rendered, output_file_path) = tokio::task::spawn_blocking(move || {
                    let output_file_path = write_article_page(
                        &site_directories,
                        rendered.meta.category,
                        &rendered.meta.slug,
                        &rendered.html,
                    )?;
                    Ok::<_, crate::artifacts::ArtifactsError>((rendered, output_file_path))
                })
                .await??;
                info!("...processed {}", output_file_path.display());
                article_metas.push(rendered.meta);
                Ok(article_metas)
            }
        })
        .await?;

    let rendered_pages = stream::iter(pages)
        .map(|parsed_file| {
            let enrich = Arc::clone(&enrich);
            render_page(parsed_file, &file_mapping, enrich)
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .try_collect::<Vec<_>>()
        .await?;

    let rendered_categories = stream::iter(categories)
        .map(|parsed_file| {
            let enrich = Arc::clone(&enrich);
            render_category(parsed_file, &file_mapping, enrich)
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .try_collect::<Vec<_>>()
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
            Ok::<_, crate::artifacts::ArtifactsError>((
                rendered_category.metadata,
                output_file_path,
            ))
        })
        .await??;
        info!("...processed {}", output_file_path.display());
        category_landings.push(metadata);
    }

    let site_artifacts = build_site_artifacts(article_metas, category_landings);
    let site_directories_for_write = site_directories.clone();
    let site_artifacts = tokio::task::spawn_blocking(move || {
        write_site_artifacts(&site_directories_for_write, &site_artifacts)?;
        Ok::<_, crate::artifacts::ArtifactsError>(site_artifacts)
    })
    .await??;

    let site_root = output_dir.join("site");
    let validation =
        tokio::task::spawn_blocking(move || validate_site_artifacts(site_root)).await??;
    info!(
        "Validated {} articles across {} categories",
        validation.article_count, validation.category_count
    );

    let processed_count = site_artifacts.article_index.len();
    let duration = start_time.elapsed();

    info!("=== Processing Summary ===");
    info!("Successfully processed: {processed_count} files");
    info!("  Skipped: {skipped} files");
    info!("  Processing time: {duration:.2?}");
    info!("Output directory: {}", output_dir.display());

    if !site_artifacts.article_index.is_empty() {
        info!("Processed files:");
        for article in &site_artifacts.article_index {
            info!("  • {} ({})", article.title.as_str(), article.slug.as_str());
        }
    }

    info!("=== Publisher Completed ===");
    Ok(())
}

fn validate_obsidian_dir(obsidian_dir: &Path) -> Result<()> {
    if !obsidian_dir.exists() {
        return Err(ObsidianError::InvalidSourceDirectory(format!(
            "directory does not exist: {}",
            obsidian_dir.display()
        )));
    }

    if !obsidian_dir.is_dir() {
        return Err(ObsidianError::InvalidSourceDirectory(format!(
            "path is not a directory: {}",
            obsidian_dir.display()
        )));
    }

    Ok(())
}
