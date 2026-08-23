use crate::artifacts::{
    SiteOutput, build_site_documents, validate_site_artifacts, write_article_page,
    write_site_documents,
};
use crate::classify::{
    ParsedArticleFile, classify_obsidian_files, ensure_category_landings,
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
use std::{path::Path, sync::Arc};
use tracing::info;

pub async fn publish(obsidian_dir: &Path, output_dir: &Path) -> Result<()> {
    publish_with_bookmark_enricher(obsidian_dir, output_dir, rich_bookmark_enricher()).await
}

#[tracing::instrument(
    name = "publish",
    skip_all,
    fields(input_dir = %obsidian_dir.display(), output_dir = %output_dir.display()),
    err
)]
pub async fn publish_with_bookmark_enricher(
    obsidian_dir: &Path,
    output_dir: &Path,
    enrich: BookmarkEnricher,
) -> Result<()> {
    validate_obsidian_dir(obsidian_dir)?;

    let start_time = std::time::Instant::now();
    info!("publish started");

    let markdown_files = scan_markdown_files(obsidian_dir)?;
    info!(file_count = markdown_files.len(), "scanned markdown files");

    let classified_files = classify_obsidian_files(markdown_files, obsidian_dir);

    info!(
        article_count = classified_files.articles.len(),
        page_count = classified_files.pages.len(),
        home_count = usize::from(classified_files.home.is_some()),
        category_count = classified_files.categories.len(),
        skipped_count = classified_files.skipped,
        error_count = classified_files.errors,
        "classified markdown files"
    );
    if classified_files.errors > 0 {
        return Err(PublishError::ContentErrors {
            count: classified_files.errors,
        });
    }

    ensure_unique_page_keys(&classified_files.pages)?;
    ensure_unique_category_landings(&classified_files.categories)?;
    ensure_category_landings(&classified_files.articles, &classified_files.categories)?;

    let link_index = links::Index::from_classified_files(&classified_files);
    let classify::ClassifiedFiles {
        articles,
        pages,
        home,
        categories,
        skipped,
        ..
    } = classified_files;

    let site_output = SiteOutput::prepare(output_dir)?;

    const CONCURRENT_LIMIT: usize = 4;

    // Drain each batch before propagating errors so started blocking writes can finish.
    let article_results = stream::iter(articles)
        .map(|parsed_file| {
            process_article(
                parsed_file,
                &link_index,
                Arc::clone(&enrich),
                site_output.clone(),
            )
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .collect::<Vec<_>>()
        .await;
    let article_metas = article_results.into_iter().collect::<Result<Vec<_>>>()?;

    let page_documents = stream::iter(pages)
        .map(|parsed_file| render_page(parsed_file, &link_index, Arc::clone(&enrich)))
        .buffer_unordered(CONCURRENT_LIMIT)
        .collect::<Vec<_>>()
        .await;

    let home_fragment = match home {
        Some(parsed_file) => Some(render_home(parsed_file, &link_index, Arc::clone(&enrich)).await),
        None => None,
    };

    let category_results = stream::iter(categories)
        .map(|parsed_file| render_category(parsed_file, &link_index, Arc::clone(&enrich)))
        .buffer_unordered(CONCURRENT_LIMIT)
        .collect::<Vec<_>>()
        .await;
    let category_landings = category_results.into_iter().collect::<Result<Vec<_>>>()?;

    let site_documents = build_site_documents(
        article_metas,
        category_landings,
        page_documents,
        home_fragment,
    )?;
    let site_output_for_write = site_output.clone();
    let site_documents = tokio::task::spawn_blocking(move || {
        write_site_documents(&site_output_for_write, &site_documents)?;
        Ok::<_, PublishError>(site_documents)
    })
    .await??;

    let site_root = site_output.root().to_path_buf();
    tokio::task::spawn_blocking(move || validate_site_artifacts(site_root)).await??;
    info!(
        article_count = site_documents.article_index.articles.len(),
        category_count = site_documents.category_count(),
        "validated site artifacts"
    );

    let processed_count = site_documents.article_index.articles.len();
    let duration = start_time.elapsed();

    info!(
        processed_count,
        skipped_count = skipped,
        processing_time_ms = duration.as_millis(),
        "publish completed"
    );

    if !site_documents.article_index.articles.is_empty() {
        for article in &site_documents.article_index.articles {
            info!(
                title = article.title.as_str(),
                slug = article.slug.as_str(),
                "processed article"
            );
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all, fields(source_key = %parsed_file.source_key), err)]
async fn process_article(
    parsed_file: ParsedArticleFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
    site_output: SiteOutput,
) -> Result<domain::ArticleMeta> {
    let article = render_article(parsed_file, link_index, enrich).await?;
    let (meta, output_file_path) = tokio::task::spawn_blocking(move || {
        let output_file_path = write_article_page(
            &site_output,
            article.meta.category,
            &article.meta.slug,
            article.body.as_str(),
        )?;
        Ok::<_, PublishError>((article.meta, output_file_path))
    })
    .await??;

    info!(output_file = %output_file_path.display(), "wrote artifact");
    Ok(meta)
}
