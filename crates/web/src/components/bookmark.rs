#![cfg(feature = "ssr")]

use crate::error::FrontendError;
use futures::{StreamExt, stream};
use lol_html::{RewriteStrSettings, element, html_content::ContentType, rewrite_str};
use scraper::{Html, Selector};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

// bookmark の最大同時取得数
const MAX_CONCURRENT_FETCHES: usize = 8; // ★ここを調整
const REQ_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
struct Ogp {
    title: String,
    description: String,
    site_name: String,
    image_url: String,
    url: String,
}

async fn fetch_ogp(url: &str) -> Result<Ogp, FrontendError> {
    #[cfg(not(target_arch = "wasm32"))]
    let client = reqwest::Client::builder()
        .timeout(REQ_TIMEOUT)
        .user_agent(concat!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
            "AppleWebKit/537.36 (KHTML, like Gecko) ",
            "Chrome/123.0.0.0 Safari/537.36"
        ))
        .default_headers({
            use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
            let mut h = HeaderMap::new();
            h.insert(
                ACCEPT,
                HeaderValue::from_static("text/html,application/xhtml+xml"),
            );
            h.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("ja,en;q=0.8"));
            h
        })
        .build()?;
    #[cfg(target_arch = "wasm32")]
    let client = reqwest::Client::new();

    let html = client.get(url).send().await?.text().await?;
    let doc = Html::parse_document(&html);
    let grab = |p: &str| {
        let sel = Selector::parse(&format!(r#"meta[property="{}"]"#, p)).unwrap();
        doc.select(&sel)
            .next()
            .and_then(|e| e.value().attr("content"))
            .unwrap_or_default()
            .to_owned()
    };
    Ok(Ogp {
        title: grab("og:title"),
        description: grab("og:description"),
        site_name: grab("og:site_name"),
        image_url: grab("og:image"),
        url: url.to_owned(),
    })
}

/// Markdown → HTML 後の文字列を受け取り、OGP カードに差し替えて返す
pub async fn rewrite_bookmarks(html_in: String) -> Result<String, FrontendError> {
    // ---------- ① URL 一覧収集（重複排除） -----------------
    let urls: HashSet<String> = {
        let sel = Selector::parse(r#"div.notion-bookmark[data-url]"#).unwrap();
        let fragment = Html::parse_fragment(&html_in);
        let mut url_set = HashSet::new();
        for n in fragment.select(&sel) {
            if let Some(u) = n.value().attr("data-url") {
                url_set.insert(u.to_owned());
            }
        }
        url_set
    }; // ここでfragmentのスコープが終了

    // ---------- ② 限定並列で OGP を取得 ---------------------
    let metas: HashMap<String, Ogp> = stream::iter(urls.into_iter())
        .map(|u| async move {
            let res = fetch_ogp(&u).await;
            (u, res)
        })
        .buffer_unordered(MAX_CONCURRENT_FETCHES) // ★同時実行数を制御
        .map(|(u, res)| {
            let meta = res.unwrap_or_else(|_e| {
                //tracing::warn!("OGP fetch failed for {}: {}", u, e);
                Ogp {
                    title: u.clone(),
                    description: String::new(),
                    site_name: String::new(),
                    image_url: String::new(),
                    url: u.clone(),
                }
            });
            (u, meta)
        })
        .collect()
        .await;

    // ---------- ③ lol_html で div をカード HTML へ置換 ------
    let rewritten = rewrite_str(
        &html_in,
        RewriteStrSettings {
            element_content_handlers: vec![
                element!(r#"div.notion-bookmark[data-url]"#, move |el| {
                    if let Some(u) = el.get_attribute("data-url") {
                        if let Some(m) = metas.get(&u) {
                            let card = format!(
                                r#"<div class="notion-bookmark">
  <a class="bookmark-link" href="{url}" target="_blank" rel="noopener">
    <div class="bookmark-content">
      <div class="bookmark-title">{title}</div>
      <div class="bookmark-description">{desc}</div>
      <div class="bookmark-domain">{site}</div>
    </div>
    <div class="bookmark-thumb" style="background-image:url('{img}')"></div>
  </a>
</div>"#,
                                url = m.url,
                                title = html_escape::encode_text(&m.title),
                                desc = html_escape::encode_text(&m.description),
                                site = html_escape::encode_text(&m.site_name),
                                img = html_escape::encode_double_quoted_attribute(
                                    if m.image_url.is_empty() {
                                        "/public/placeholder.svg" // フォールバック
                                    } else {
                                        &m.image_url
                                    }
                                ),
                            );
                            el.replace(&card, ContentType::Html);
                        }
                    }
                    Ok(())
                })
                .into(),
            ],
            ..RewriteStrSettings::default()
        },
    )?;

    Ok(rewritten)
}
