use crate::config::Config;
use crate::error::Result;
use crate::database::extract_pages;
use crate::markdown::extract_blocks;
use crate::models::{BlockInfo, PageInfo};
use governor::{DefaultDirectRateLimiter, Quota};
use reqwest::Client;
use serde_json::{Value, json};
use std::num::NonZeroU32;

/// ページ分割されたデータを処理する構造体
#[derive(Debug)]
struct Pagination<T> {
    contents: Vec<T>,
    next_cursor: Option<String>,
}

pub struct NotionClient {
    pub client: Client,
    pub config: Config,
    limiter: DefaultDirectRateLimiter,
}

/// Notion APIクライアント
/// インターフェースとなる関数は、newとquery_database, query_pageのみ
/// 実際のHTTPリクエスト処理は内部で行っており、具体的な処理はハードコードされている
impl NotionClient {
    pub fn new(config: Config) -> Self {
        // 1秒あたり2リクエストのレートリミッターを設定
        // 実際には3リクエスト可能だが安全のため、2リクエストに設定
        let quota =
            Quota::per_second(NonZeroU32::new(2).unwrap()).allow_burst(NonZeroU32::new(2).unwrap());
        let limiter = governor::RateLimiter::direct(quota);
        NotionClient {
            client: Client::new(),
            config,
            limiter,
        }
    }

    /// GETリクエスト
    async fn http_get(&self, url: &str) -> Result<Value> {
        self.limiter.until_ready().await;
        let res = self
            .client
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.notion_token),
            )
            .header("Notion-Version", "2022-06-28")
            .send()
            .await?;
        Ok(res.json().await?)
    }

    /// POSTリクエスト
    async fn http_post(&self, url: &str, body: &str) -> Result<Value> {
        self.limiter.until_ready().await;
        let res = self
            .client
            .post(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.notion_token),
            )
            .header("Notion-Version", "2022-06-28")
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await?;
        Ok(res.json().await?)
    }

    pub async fn query_database(&self) -> Result<Vec<PageInfo>> {
        let mut all_pages = Vec::new();
        let mut next_cursor: Option<String> = None;
        loop {
            let pagination = self.query_database_chunk(next_cursor.as_ref()).await?;
            all_pages.extend(pagination.contents);
            if let Some(cursor) = pagination.next_cursor {
                next_cursor = Some(cursor);
            } else {
                break;
            }
        }
        Ok(all_pages)
    }

    async fn query_database_chunk(
        &self,
        next_cursor: Option<&String>,
    ) -> Result<Pagination<PageInfo>> {
        // ページのステータスが「完了」のものを取得するクエリ
        // その他のフィルター条件はここに記述可能
        let mut body_obj = json!({
            "filter": {
                "property": "ステータス",
                "status": { "equals": "完了" }
            }
        });
        if let Some(cursor) = next_cursor {
            body_obj
                .as_object_mut()
                .unwrap()
                .insert("start_cursor".to_string(), json!(cursor));
        }
        let body_str = body_obj.to_string();

        let url = format!(
            "https://api.notion.com/v1/databases/{}/query",
            self.config.database_id
        );
        let json_response = self.http_post(&url, &body_str).await?;
        let next_cursor = if json_response
            .get("has_more")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            json_response
                .get("next_cursor")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };
        let pages = extract_pages(&json_response)?;
        Ok(Pagination {
            contents: pages,
            next_cursor,
        })
    }

    /// ページの子ブロックを取得し、Markdownに変換してファイルに出力する
    pub async fn query_page(&self, page: &PageInfo) -> Result<Vec<BlockInfo>> {
        let mut all_blocks = Vec::new();
        let mut next_cursor: Option<String> = None;
        loop {
            let pagination = self.query_page_chunk(page, next_cursor.as_ref()).await?;
            all_blocks.extend(pagination.contents);
            if let Some(cursor) = pagination.next_cursor {
                next_cursor = Some(cursor);
            } else {
                break;
            }
        }
        Ok(all_blocks)
    }

    async fn query_page_chunk(
        &self,
        page: &PageInfo,
        next_cursor: Option<&String>,
    ) -> Result<Pagination<BlockInfo>> {
        let url = if let Some(cursor) = next_cursor {
            format!(
                "https://api.notion.com/v1/blocks/{}/children?start_cursor={}",
                page.id, cursor
            )
        } else {
            format!("https://api.notion.com/v1/blocks/{}/children", page.id)
        };
        let json_response = self.http_get(&url).await?;
        let next_cursor = if json_response
            .get("has_more")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            json_response
                .get("next_cursor")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        let blocks = extract_blocks(&json_response)?;
        Ok(Pagination {
            contents: blocks,
            next_cursor,
        })
    }
}
