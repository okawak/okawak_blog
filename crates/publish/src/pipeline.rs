use crate::artifacts::{
    SiteDirectories, build_site_artifacts, validate_site_artifacts, write_article_page,
    write_category_page, write_home_fragment, write_page_document, write_site_artifacts,
};
use crate::classify::{
    ParsedArticleFile, ParsedCategoryFile, ParsedHomeFile, ParsedPageFile, classify_obsidian_files,
    ensure_unique_category_landings, ensure_unique_page_keys,
};
use crate::error::{PublishError, Result};
use crate::render::{
    BookmarkEnricher, render_article, render_category, render_home, render_page,
    rich_bookmark_enricher,
};
use crate::vault::{scan_markdown_files, validate_obsidian_dir};
use crate::{classify, links};
use futures::{StreamExt, stream};
use log::info;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

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

    // Drain each batch before propagating errors so started blocking writes can finish.
    let article_results = stream::iter(articles)
        .map(|parsed_file| {
            process_article(
                parsed_file,
                &link_index,
                Arc::clone(&enrich),
                site_directories.clone(),
            )
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .collect::<Vec<_>>()
        .await;
    let article_metas = article_results.into_iter().collect::<Result<Vec<_>>>()?;

    let page_results = stream::iter(pages)
        .map(|parsed_file| {
            process_page(
                parsed_file,
                &link_index,
                Arc::clone(&enrich),
                site_directories.clone(),
            )
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .collect::<Vec<_>>()
        .await;
    page_results.into_iter().collect::<Result<Vec<_>>>()?;

    if let Some(parsed_file) = home {
        process_home(
            parsed_file,
            &link_index,
            Arc::clone(&enrich),
            site_directories.clone(),
        )
        .await?;
    }

    let category_results = stream::iter(categories)
        .map(|parsed_file| {
            process_category(
                parsed_file,
                &link_index,
                Arc::clone(&enrich),
                site_directories.clone(),
            )
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .collect::<Vec<_>>()
        .await;
    let category_landings = category_results.into_iter().collect::<Result<Vec<_>>>()?;

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

async fn process_article(
    parsed_file: ParsedArticleFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
    site_directories: SiteDirectories,
) -> Result<domain::ArticleMeta> {
    let rendered = render_article(parsed_file, link_index, enrich).await?;
    run_artifact_write(move || {
        let output_file_path = write_article_page(
            &site_directories,
            rendered.meta.category,
            &rendered.meta.slug,
            &rendered.html,
        )?;
        Ok((rendered.meta, output_file_path))
    })
    .await
}

async fn process_page(
    parsed_file: ParsedPageFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
    site_directories: SiteDirectories,
) -> Result<()> {
    let rendered = render_page(parsed_file, link_index, enrich).await?;
    run_artifact_write(move || {
        let output_file_path = write_page_document(&site_directories, &rendered.document)?;
        Ok(((), output_file_path))
    })
    .await
}

async fn process_home(
    parsed_file: ParsedHomeFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
    site_directories: SiteDirectories,
) -> Result<()> {
    let rendered = render_home(parsed_file, link_index, enrich).await?;
    run_artifact_write(move || {
        let output_file_path = write_home_fragment(&site_directories, &rendered)?;
        Ok(((), output_file_path))
    })
    .await
}

async fn process_category(
    parsed_file: ParsedCategoryFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
    site_directories: SiteDirectories,
) -> Result<domain::CategoryLandingMeta> {
    let rendered = render_category(parsed_file, link_index, enrich).await?;
    run_artifact_write(move || {
        let output_file_path =
            write_category_page(&site_directories, rendered.meta.category, &rendered.html)?;
        Ok((rendered.meta, output_file_path))
    })
    .await
}

async fn run_artifact_write<T>(
    write: impl FnOnce() -> Result<(T, PathBuf)> + Send + 'static,
) -> Result<T>
where
    T: Send + 'static,
{
    let (result, output_file_path) = tokio::task::spawn_blocking(write).await??;
    info!("...processed {}", output_file_path.display());
    Ok(result)
}
