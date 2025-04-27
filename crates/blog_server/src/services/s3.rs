#![cfg(feature = "ssr")]

use crate::error::AppError;
use crate::models::article::{Article, ArticleSummary};
use aws_sdk_s3::Client;
use chrono::NaiveDate;
use leptos::context::use_context;
use log;
use regex::Regex;
use serde::Deserialize;

/// カテゴリー内の記事一覧を取得する
pub async fn list_articles(category: &str) -> Result<Vec<ArticleSummary>, AppError> {
    let client: Client = use_context()
        .ok_or_else(|| AppError::S3Error("S3クライアントの初期化に失敗".to_string()))?;

    let bucket_name =
        std::env::var("AWS_BUCKET_NAME").unwrap_or_else(|_| "bucket-name".to_string());

    // S3バケット内のオブジェクトをリストアップ
    // 指定したカテゴリのフォルダ内のファイルを検索
    let prefix = format!("{}/", category);
    log::info!(
        "S3バケット '{}' のプレフィックス '{}' で記事を検索",
        &bucket_name,
        &prefix
    );

    let list_objects_result = client
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(&prefix)
        .send()
        .await
        .map_err(|e| {
            log::error!("S3オブジェクトリスト取得エラー: {}", e);
            AppError::S3Error(format!("S3オブジェクトリスト取得エラー: {}", e))
        })?;

    let objects: Vec<_> = list_objects_result
        .contents
        .unwrap_or_default()
        .into_iter()
        .filter(|obj| {
            match &obj.key {
                Some(key) => {
                    // フォルダマーカー（末尾 '/'）を除外＆.md ファイルだけ
                    !key.ends_with('/') && key.ends_with(".md")
                }
                None => false,
            }
        })
        .collect();

    log::info!(
        "カテゴリ '{}' で {} 件のオブジェクトが見つかりました",
        category,
        objects.len()
    );

    let mut articles = Vec::new();
    for object in objects {
        if let Some(key) = object.key {
            // キー（パス）から記事IDとスラッグを抽出
            let parts: Vec<&str> = key.split('/').collect();
            if parts.len() < 2 {
                log::warn!("不正なキー形式: {}", key);
                continue;
            }

            let filename = parts[parts.len() - 1];
            let slug = filename.trim_end_matches(".md");

            // メタデータを取得して記事サマリーを作成
            match get_article_metadata(&client, &key).await {
                Ok(metadata) => {
                    let summary = ArticleSummary {
                        id: format!("{}/{}", category, slug),
                        title: metadata.title,
                        slug: slug.to_string(),
                        category: category.to_string(),
                        group: metadata.group,
                        priority_level: metadata.priority_level,
                        summary: metadata.summary,
                        tags: metadata.tags,
                        published_date: metadata.published_at,
                        published_at: metadata.published_at.format("%Y年%m月%d日").to_string(),
                        updated_at: metadata.updated_at.format("%Y年%m月%d日").to_string(),
                    };
                    articles.push(summary);
                }
                Err(e) => {
                    log::warn!("記事 '{}' のメタデータ取得に失敗: {}", key, e);
                    continue;
                }
            }
        }
    }

    // 投稿日時の降順でソート
    articles.sort_by(|a, b| b.published_date.cmp(&a.published_date));

    Ok(articles)
}

pub async fn fetch_latest_articles(category: String) -> Result<Vec<ArticleSummary>, AppError> {
    // 全カテゴリーから最新記事を集める
    let categories;
    if category.is_empty() {
        categories = vec!["tech", "daily", "statistics", "physics"];
    } else {
        categories = vec![&category];
    }
    let mut all_articles = Vec::new();

    for category in categories {
        if let Ok(mut v) = list_articles(category).await {
            all_articles.append(&mut v);
        } else {
            log::error!("カテゴリー {category} の記事取得に失敗");
        }
    }

    // 投稿日時の降順でソート
    all_articles.sort_by(|a, b| b.published_date.cmp(&a.published_date));

    // 最大10件に制限
    Ok(all_articles.into_iter().take(10).collect())
}

/// 記事の詳細を取得する
pub async fn get_article(category: &str, slug: &str) -> Result<Article, AppError> {
    let client: Client = use_context()
        .ok_or_else(|| AppError::S3Error("S3クライアントの初期化に失敗".to_string()))?;

    let bucket_name =
        std::env::var("AWS_BUCKET_NAME").unwrap_or_else(|_| "bucket-name".to_string());

    let key = format!("{}/{}.md", category, slug);
    log::info!("S3バケット '{}' から記事 '{}' を取得", &bucket_name, key);

    // S3からMarkdownファイルを取得
    let get_object_result = match client
        .get_object()
        .bucket(&bucket_name)
        .key(&key)
        .send()
        .await
    {
        Ok(result) => result,
        Err(e) => return Err(AppError::S3Error(format!("記事の取得に失敗: {}", e))),
    };

    // ファイル内容を文字列として読み込む
    let byte_stream = get_object_result.body;
    let bytes = match byte_stream.collect().await {
        Ok(bytes) => bytes.into_bytes(),
        Err(e) => {
            return Err(AppError::S3Error(format!(
                "ファイル内容の読み込みに失敗: {}",
                e
            )));
        }
    };

    let content = match std::str::from_utf8(&bytes) {
        Ok(content) => content,
        Err(e) => {
            return Err(AppError::S3Error(format!(
                "UTF-8文字列への変換に失敗: {}",
                e
            )));
        }
    };

    // Markdownファイルを解析
    match parse_markdown_article(content, category, slug) {
        Ok(article) => Ok(article),
        Err(e) => Err(AppError::S3Error(format!(
            "Markdownファイルの解析に失敗: {}",
            e
        ))),
    }
}

/// Markdownファイルのメタデータを取得
async fn get_article_metadata(client: &Client, key: &str) -> Result<ArticleMetadata, AppError> {
    let bucket_name =
        std::env::var("AWS_BUCKET_NAME").unwrap_or_else(|_| "bucket-name".to_string());
    // S3オブジェクトをGET
    let get_object_result = match client
        .get_object()
        .bucket(&bucket_name)
        .key(key)
        .send()
        .await
    {
        Ok(result) => result,
        Err(e) => {
            return Err(AppError::MarkdownError(format!(
                "ファイルの取得に失敗: {}",
                e
            )));
        }
    };

    // ファイルの先頭部分のみを取得して解析
    let byte_stream = get_object_result.body;
    let bytes = match byte_stream.collect().await {
        Ok(bytes) => bytes.into_bytes(),
        Err(e) => {
            return Err(AppError::MarkdownError(format!(
                "ファイル内容の読み込みに失敗: {}",
                e
            )));
        }
    };

    let content = match std::str::from_utf8(&bytes) {
        Ok(content) => content,
        Err(e) => {
            return Err(AppError::MarkdownError(format!(
                "UTF-8文字列への変換に失敗: {}",
                e
            )));
        }
    };

    // フロントマターを解析
    extract_metadata_from_markdown(content)
}

/// Markdownファイルからフロントマターを抽出してメタデータを構築
fn extract_metadata_from_markdown(content: &str) -> Result<ArticleMetadata, AppError> {
    // frontmatter, between +++
    let front_matter_pattern = Regex::new(r"^\+\+\+([\s\S]*?)\+\+\+").unwrap();

    let front_matter = match front_matter_pattern.captures(content) {
        Some(captures) => {
            if let Some(matched) = captures.get(1) {
                matched.as_str()
            } else {
                return Err(AppError::MarkdownError(
                    "フロントマターが見つかりませんでした".to_string(),
                ));
            }
        }
        None => {
            return Err(AppError::MarkdownError(
                "フロントマターが見つかりませんでした".to_string(),
            ));
        }
    };

    // TOMLとしてパース
    let parsed = match toml::from_str::<ArticleMetadataToml>(front_matter) {
        Ok(parsed) => parsed,
        Err(e) => {
            return Err(AppError::MarkdownError(format!(
                "フロントマターの解析に失敗: {}",
                e
            )));
        }
    };

    // 日付文字列をNaiveDateに変換
    let published_date = match NaiveDate::parse_from_str(
        &parsed.created_time.split('T').next().unwrap_or(""),
        "%Y-%m-%d",
    ) {
        Ok(date) => date,
        Err(_) => chrono::Local::now().naive_local().date(),
    };

    // 更新日時をNaiveDateに変換
    let updated_date = match NaiveDate::parse_from_str(
        &parsed.last_edited_time.split('T').next().unwrap_or(""),
        "%Y-%m-%d",
    ) {
        Ok(date) => date,
        Err(_) => chrono::Local::now().naive_local().date(),
    };

    Ok(ArticleMetadata {
        id: parsed.id,
        title: parsed.title,
        category: parsed.category,
        group: parsed.group,
        priority_level: parsed.priority_level,
        tags: parsed.tags,
        summary: parsed.summary,
        published_at: published_date,
        updated_at: updated_date,
    })
}

/// Markdownファイルを解析して記事オブジェクトを構築
fn parse_markdown_article(content: &str, category: &str, slug: &str) -> Result<Article, AppError> {
    // フロントマターからメタデータを抽出
    let metadata = extract_metadata_from_markdown(content)?;

    // 本文部分を抽出（フロントマター以降のすべて）
    let front_matter_pattern = Regex::new(r"^\+\+\+([\s\S]*?)\+\+\+").unwrap();
    let body = front_matter_pattern.replace(content, "").trim().to_string();

    // 記事オブジェクトを構築
    let article = Article {
        id: format!("{}/{}", category, slug),
        title: metadata.title,
        slug: slug.to_string(),
        category: category.to_string(),
        group: metadata.group,
        priority_level: metadata.priority_level,
        summary: metadata.summary,
        tags: metadata.tags,
        published_date: metadata.published_at,
        published_at: metadata.published_at.format("%Y年%m月%d日").to_string(),
        updated_at: metadata.updated_at.format("%Y年%m月%d日").to_string(),
        content: body,
    };

    Ok(article)
}

/// HTMLタグを除去する簡易関数
//fn strip_html_tags(html: &str) -> String {
//    let re = Regex::new(r"<[^>]*>").unwrap();
//    re.replace_all(html, "").to_string()
//}

/// S3ファイルから解析したメタデータ
/// 日付の処理が加えられている
#[allow(dead_code)]
struct ArticleMetadata {
    id: String,
    title: String,
    category: String,
    group: String,
    priority_level: i32,
    tags: Vec<String>,
    summary: String,
    published_at: NaiveDate,
    updated_at: NaiveDate,
}

/// フロントマターをパースするための構造体
#[derive(Deserialize)]
struct ArticleMetadataToml {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    group: String,
    #[serde(default)]
    priority_level: i32,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    created_time: String,
    #[serde(default)]
    last_edited_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    _status: Option<String>,
}
