use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, config::Region};
use chrono::NaiveDate;
use serde::Deserialize;
use crate::models::article::{Article, ArticleSummary};
use crate::error::AppError;
use log;
use regex::Regex;

/// S3バケット名
const BUCKET_NAME: &str = "blog-okawak-app";

/// S3クライアントを初期化する
async fn create_s3_client() -> Result<Client, AppError> {
    let region_provider = RegionProviderChain::first_try(Region::new("ap-northeast-1"));

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    Ok(Client::new(&config))
}

/// カテゴリー内の記事一覧を取得する
pub async fn list_articles(category: &str) -> Result<Vec<ArticleSummary>, String> {
    let client = match create_s3_client().await {
        Ok(client) => client,
        Err(e) => return Err(format!("S3クライアントの初期化に失敗: {}", e)),
    };

    // S3バケット内のオブジェクトをリストアップ
    // 指定したカテゴリのフォルダ内のファイルを検索
    let prefix = format!("{}/", category);
    log::info!("S3バケット '{}' のプレフィックス '{}' で記事を検索", BUCKET_NAME, prefix);

    let list_objects_result = match client
        .list_objects_v2()
        .bucket(BUCKET_NAME)
        .prefix(&prefix)
        .send()
        .await {
            Ok(result) => result,
            Err(e) => return Err(format!("S3バケットのオブジェクトリスト取得に失敗: {}", e)),
        };

    let objects = match list_objects_result.contents {
        Some(objects) => objects,
        None => return Ok(vec![]), // オブジェクトが存在しない場合は空のリストを返す
    };

    log::info!("カテゴリ '{}' で {} 件のオブジェクトが見つかりました", category, objects.len());

    let mut articles = Vec::new();
    for object in objects {
        if let Some(key) = object.key {
            // .mdファイルのみを処理
            if !key.ends_with(".md") {
                continue;
            }

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
                        excerpt: metadata.description.clone().unwrap_or_default(),
                        thumbnail_url: metadata.thumbnail_url,
                        tags: metadata.tags,
                        published_at: metadata.published_at,
                        date_formatted: metadata.published_at.format("%Y年%m月%d日").to_string(),
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
    articles.sort_by(|a, b| b.published_at.cmp(&a.published_at));

    Ok(articles)
}

/// 記事の詳細を取得する
pub async fn get_article(category: &str, slug: &str) -> Result<Article, String> {
    let client = match create_s3_client().await {
        Ok(client) => client,
        Err(e) => return Err(format!("S3クライアントの初期化に失敗: {}", e)),
    };

    let key = format!("{}/{}.md", category, slug);
    log::info!("S3バケット '{}' から記事 '{}' を取得", BUCKET_NAME, key);

    // S3からMarkdownファイルを取得
    let get_object_result = match client
        .get_object()
        .bucket(BUCKET_NAME)
        .key(&key)
        .send()
        .await {
            Ok(result) => result,
            Err(e) => return Err(format!("記事の取得に失敗: {}", e)),
        };

    // ファイル内容を文字列として読み込む
    let byte_stream = get_object_result.body;
    let bytes = match byte_stream.collect().await {
        Ok(bytes) => bytes.into_bytes(),
        Err(e) => return Err(format!("ファイル内容の読み込みに失敗: {}", e)),
    };

    let content = match std::str::from_utf8(&bytes) {
        Ok(content) => content,
        Err(e) => return Err(format!("UTF-8文字列への変換に失敗: {}", e)),
    };

    // Markdownファイルを解析
    match parse_markdown_article(content, category, slug) {
        Ok(article) => Ok(article),
        Err(e) => Err(format!("Markdownファイルの解析に失敗: {}", e)),
    }
}

/// Markdownファイルのメタデータを取得
async fn get_article_metadata(client: &Client, key: &str) -> Result<ArticleMetadata, String> {
    // S3オブジェクトをGET
    let get_object_result = match client
        .get_object()
        .bucket(BUCKET_NAME)
        .key(key)
        .send()
        .await {
            Ok(result) => result,
            Err(e) => return Err(format!("ファイルの取得に失敗: {}", e)),
        };

    // ファイルの先頭部分のみを取得して解析
    let byte_stream = get_object_result.body;
    let bytes = match byte_stream.collect().await {
        Ok(bytes) => bytes.into_bytes(),
        Err(e) => return Err(format!("ファイル内容の読み込みに失敗: {}", e)),
    };

    let content = match std::str::from_utf8(&bytes) {
        Ok(content) => content,
        Err(e) => return Err(format!("UTF-8文字列への変換に失敗: {}", e)),
    };

    // フロントマターを解析
    extract_metadata_from_markdown(content)
}

/// Markdownファイルからフロントマターを抽出してメタデータを構築
fn extract_metadata_from_markdown(content: &str) -> Result<ArticleMetadata, String> {
    // フロントマターを検索
    // フロントマターは+++で囲まれている（TOMLフォーマット）
    let front_matter_pattern = Regex::new(r"^\+\+\+([\s\S]*?)\+\+\+").unwrap();

    let front_matter = match front_matter_pattern.captures(content) {
        Some(captures) => {
            if let Some(matched) = captures.get(1) {
                matched.as_str()
            } else {
                return Err("フロントマターが見つかりませんでした".to_string());
            }
        }
        None => return Err("フロントマターが見つかりませんでした".to_string()),
    };

    // TOMLとしてパース
    let parsed = match toml::from_str::<ArticleMetadataToml>(front_matter) {
        Ok(parsed) => parsed,
        Err(e) => return Err(format!("フロントマターの解析に失敗: {}", e)),
    };

    // 日付文字列をNaiveDateに変換
    let published_date = match NaiveDate::parse_from_str(&parsed.created_time.split('T').next().unwrap_or(""), "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => chrono::Local::now().naive_local().date(),
    };

    // 更新日時を変換（指定されている場合）
    let updated_date = parsed.last_edited_time.as_ref()
        .and_then(|date_str| {
            NaiveDate::parse_from_str(date_str.split('T').next().unwrap_or(""), "%Y-%m-%d").ok()
        });

    Ok(ArticleMetadata {
        title: parsed.title,
        description: parsed.description,
        published_at: published_date,
        updated_at: updated_date,
        tags: parsed.tags,
        thumbnail_url: parsed.thumbnail_url,
    })
}

/// Markdownファイルを解析して記事オブジェクトを構築
fn parse_markdown_article(content: &str, category: &str, slug: &str) -> Result<Article, String> {
    // フロントマターからメタデータを抽出
    let metadata = extract_metadata_from_markdown(content)?;

    // 本文部分を抽出（フロントマター以降のすべて）
    let front_matter_pattern = Regex::new(r"^\+\+\+([\s\S]*?)\+\+\+").unwrap();
    let body = front_matter_pattern
        .replace(content, "")
        .trim()
        .to_string();

    // 記事の最初の150文字を抜粋として使用（HTMLタグを除去）
    let plain_content = strip_html_tags(&body);
    let excerpt = if plain_content.len() > 150 {
        format!("{}...", &plain_content[..150])
    } else {
        plain_content
    };

    // OGP画像はサムネイルURLがあればそれを使用、なければデフォルト画像
    let og_image = metadata.thumbnail_url.clone();

    // 記事オブジェクトを構築
    let article = Article {
        id: format!("{}/{}", category, slug),
        title: metadata.title,
        slug: slug.to_string(),
        category: category.to_string(),
        content: body,
        excerpt,
        published_at: metadata.published_at,
        updated_at: metadata.updated_at,
        description: metadata.description.unwrap_or_default(),
        tags: metadata.tags,
        thumbnail_url: metadata.thumbnail_url,
        og_image,
        published: true, // デフォルトで公開する
    };

    Ok(article)
}

/// HTMLタグを除去する簡易関数
fn strip_html_tags(html: &str) -> String {
    let re = Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(html, "").to_string()
}

/// S3ファイルから解析したメタデータ
struct ArticleMetadata {
    title: String,
    description: Option<String>,
    published_at: NaiveDate,
    updated_at: Option<NaiveDate>,
    tags: Vec<String>,
    thumbnail_url: Option<String>,
}

/// フロントマターをパースするための構造体
#[derive(Deserialize)]
struct ArticleMetadataToml {
    title: String,
    #[serde(default)]
    description: Option<String>,
    created_time: String,
    #[serde(default)]
    last_edited_time: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    thumbnail_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}
