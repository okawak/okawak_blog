use crate::artifacts::{
    SiteOutput, build_site_documents, write_article_page, write_site_documents,
};
use crate::classify::{ParsedArticleFile, ensure_category_landings};
use crate::error::{PublishError, Result};
use crate::render::{
    BookmarkEnricher, render_article, render_category, render_home, render_page,
    rich_bookmark_enricher,
};
use crate::{classify, input, links};
use domain::{Locale, SiteLocalesDocument};
use futures::{StreamExt, stream};
use std::{path::Path, sync::Arc};
use tracing::info;

pub async fn publish(content_dir: &Path, output_dir: &Path) -> Result<()> {
    publish_with_bookmark_enricher(content_dir, output_dir, rich_bookmark_enricher()).await
}

#[tracing::instrument(name = "publish", skip_all, err)]
pub async fn publish_with_bookmark_enricher(
    content_dir: &Path,
    output_dir: &Path,
    enrich: BookmarkEnricher,
) -> Result<()> {
    let documents = input::read(content_dir)?;
    let japanese = input::eligible(&documents, Locale::Ja);
    let english = input::eligible(&documents, Locale::En);
    let asset_names = links::assets(&japanese)?
        .union(&links::assets(&english)?)
        .cloned()
        .collect::<Vec<_>>();
    for name in &asset_names {
        let path = content_dir.join("assets").join(name);
        if !std::fs::symlink_metadata(path)?.file_type().is_file() {
            return Err(PublishError::Parse(
                "referenced public asset must be a regular file".into(),
            ));
        }
    }
    let ja_files = classify::classify(japanese.clone());
    if ja_files.articles.is_empty() {
        return Err(PublishError::NoArticles);
    }
    if !ja_files.pages.iter().any(|p| p.page.as_str() == "about") {
        return Err(PublishError::MissingAboutPage);
    }
    ensure_category_landings(&ja_files.articles, &ja_files.categories)?;
    let ja_links = links::Index::new(&japanese, &japanese);
    let en_links = links::Index::new(&japanese, &english);
    ja_links.validate(&japanese)?;
    en_links.validate(&english)?;
    let parent = output_dir
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let stage = tempfile::Builder::new()
        .prefix(".publish-stage-")
        .tempdir_in(parent)?;
    let mut locales = SiteLocalesDocument::default();
    for (locale, docs, index) in [
        (Locale::Ja, japanese, ja_links),
        (Locale::En, english, en_links),
    ] {
        if locale == Locale::En && docs.is_empty() {
            continue;
        }
        locales.routes.entry("/".into()).or_default().push(locale);
        for doc in &docs {
            let mut meta = doc.meta.clone();
            meta.locale = Locale::Ja;
            let available = locales.routes.entry(meta.path()).or_default();
            if !available.contains(&locale) {
                available.push(locale);
            }
        }
        publish_locale(docs, &index, stage.path(), locale, Arc::clone(&enrich)).await?;
    }
    locales.validate()?;
    std::fs::write(
        stage.path().join("site/locales.json"),
        serde_json::to_vec(&locales)?,
    )?;
    if !asset_names.is_empty() {
        let dest = stage.path().join("site/content-assets");
        std::fs::create_dir_all(&dest)?;
        for name in asset_names {
            std::fs::copy(content_dir.join("assets").join(&name), dest.join(name))?;
        }
    }
    std::fs::create_dir_all(output_dir)?;
    let destination = output_dir.join("site");
    let backup = output_dir.join(".site-backup");
    if backup.exists() {
        return Err(PublishError::Parse(
            "recover .site-backup before publishing".into(),
        ));
    }
    let existed = destination.exists();
    if existed {
        std::fs::rename(&destination, &backup)?;
    }
    if let Err(error) = std::fs::rename(stage.path().join("site"), &destination) {
        if existed {
            std::fs::rename(&backup, &destination)?;
        }
        return Err(error.into());
    }
    if existed {
        std::fs::remove_dir_all(backup)?;
    }
    Ok(())
}

async fn publish_locale(
    documents: Vec<input::Document>,
    link_index: &links::Index,
    output_dir: &Path,
    locale: Locale,
    enrich: BookmarkEnricher,
) -> Result<()> {
    let start_time = std::time::Instant::now();
    let classify::ClassifiedFiles {
        articles,
        pages,
        home,
        categories,
    } = classify::classify(documents);
    let site_output = SiteOutput::prepare_locale(output_dir, locale)?;

    const CONCURRENT_LIMIT: usize = 4;

    // Drain each batch before propagating errors so started blocking writes can finish.
    let article_results = stream::iter(articles)
        .map(|parsed_file| {
            process_article(
                parsed_file,
                link_index,
                Arc::clone(&enrich),
                site_output.clone(),
            )
        })
        .buffer_unordered(CONCURRENT_LIMIT)
        .collect::<Vec<_>>()
        .await;
    let article_metas = article_results.into_iter().collect::<Result<Vec<_>>>()?;

    let page_documents = stream::iter(pages)
        .map(|parsed_file| render_page(parsed_file, link_index, Arc::clone(&enrich)))
        .buffer_unordered(CONCURRENT_LIMIT)
        .collect::<Vec<_>>()
        .await;

    let home_fragment = match home {
        Some(parsed_file) => Some(render_home(parsed_file, link_index, Arc::clone(&enrich)).await),
        None => None,
    };

    let category_results = stream::iter(categories)
        .map(|parsed_file| render_category(parsed_file, link_index, Arc::clone(&enrich)))
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

    info!(
        article_count = site_documents.article_index.articles.len(),
        category_count = site_documents.category_count(),
        locale = %locale,
        processing_time_ms = start_time.elapsed().as_millis(),
        "publish completed"
    );

    Ok(())
}

#[tracing::instrument(skip_all, fields(id = %parsed_file.slug), err)]
async fn process_article(
    parsed_file: ParsedArticleFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
    site_output: SiteOutput,
) -> Result<domain::ArticleMeta> {
    let article = render_article(parsed_file, link_index, enrich).await?;
    tokio::task::spawn_blocking(move || {
        write_article_page(
            &site_output,
            article.meta.category,
            &article.meta.slug,
            article.body.as_str(),
        )?;
        Ok::<_, PublishError>(article.meta)
    })
    .await?
}
