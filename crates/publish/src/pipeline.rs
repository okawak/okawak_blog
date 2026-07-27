use crate::artifacts::{
    SiteDirectories, build_site_artifacts, validate_site_artifacts, write_article_page,
    write_category_page, write_home_fragment, write_page_document, write_site_artifacts,
};
use crate::classify::{
    classify_obsidian_files, ensure_unique_category_landings, ensure_unique_page_keys,
};
use crate::error::{PublishError, Result};
use crate::render::{
    BookmarkEnricher, render_article, render_category, render_home, render_page,
    rich_bookmark_enricher,
};
use crate::vault::{scan_markdown_files, validate_obsidian_dir};
use crate::{classify, links};
use futures::{StreamExt, TryStreamExt, stream};
use log::info;
use std::{path::Path, sync::Arc};

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

    let markdown_files = scan_markdown_files(obsidian_dir)?;
    info!("Found {} markdown files", markdown_files.len());

    let classify::ClassifiedFiles {
        articles,
        pages,
        home,
        categories,
        skipped,
        errors,
    } = classify_obsidian_files(markdown_files, obsidian_dir);

    info!("Valid article files: {}", articles.len());
    info!("Valid page files: {}", pages.len());
    info!("Valid home file: {}", usize::from(home.is_some()));
    info!("Valid category files: {}", categories.len());
    info!("Skipped files: {skipped}");
    if errors > 0 {
        return Err(PublishError::ContentErrors { count: errors });
    }

    ensure_unique_page_keys(&pages)?;
    ensure_unique_category_landings(&categories)?;

    let link_index = links::Index::from_articles(&articles);
    let site_directories = SiteDirectories::prepare(output_dir)?;

    const CONCURRENT_LIMIT: usize = 4;

    let article_metas = stream::iter(articles)
        .map(|parsed_file| {
            let enrich = Arc::clone(&enrich);
            render_article(parsed_file, &link_index, enrich)
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
                    Ok::<_, PublishError>((rendered, output_file_path))
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
            render_page(parsed_file, &link_index, enrich)
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .try_collect::<Vec<_>>()
        .await?;

    let rendered_home = match home {
        Some(parsed_file) => {
            Some(render_home(parsed_file, &link_index, Arc::clone(&enrich)).await?)
        }
        None => None,
    };

    let rendered_categories = stream::iter(categories)
        .map(|parsed_file| {
            let enrich = Arc::clone(&enrich);
            render_category(parsed_file, &link_index, enrich)
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

    if let Some(rendered_home) = rendered_home {
        let site_directories = site_directories.clone();
        let output_file_path = tokio::task::spawn_blocking(move || {
            write_home_fragment(&site_directories, &rendered_home)
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
            Ok::<_, PublishError>((rendered_category.metadata, output_file_path))
        })
        .await??;
        info!("...processed {}", output_file_path.display());
        category_landings.push(metadata);
    }

    let site_artifacts = build_site_artifacts(article_metas, category_landings);
    let site_directories_for_write = site_directories.clone();
    let site_artifacts = tokio::task::spawn_blocking(move || {
        write_site_artifacts(&site_directories_for_write, &site_artifacts)?;
        Ok::<_, PublishError>(site_artifacts)
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
