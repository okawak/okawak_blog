use crate::config::Config;
use crate::database::extract_pages;
use crate::error::Result;
use crate::markdown::extract_blocks;
use crate::models::{BlockInfo, PageInfo};
use governor::{DefaultDirectRateLimiter, Quota};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::{Client, Url};
use serde::Serialize;
use std::num::NonZeroU32;

/// Notion APIのページ分割レスポンス
/// 受け取るJSONは以下のような形式
/// Tはデータベースからの場合はPageInfo、ページからの場合はBlockInfo
/// {
///   "object": "list",
///   "results": [
///     ...
///   ],
///   "next_cursor": null,
///   "has_more": false,
///   "type": "page_or_database",
///   "page_or_database": {},
///   "request_id": "***"
/// }
#[derive(Debug)]
struct NotionResponse<T> {
    results: Vec<T>,
    next_cursor: Option<String>,
}

/// クエリボディ全体の型定義
/// 送るJSONは以下のような形式(フィルター条件はハードコードしている)
/// {
///   "filter": {
///     "property": "ステータス",
///       "status": {
///         "equals": "完了"
///       }
///    }
///   "start_cursor": "***"
/// }
#[derive(Serialize)]
struct QueryBody<'a> {
    filter: QueryFilter<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_cursor: Option<&'a str>,
}
#[derive(Serialize)]
struct QueryFilter<'a> {
    property: &'static str,
    status: StatusEq<'a>,
}
#[derive(Serialize)]
struct StatusEq<'a> {
    equals: &'a str,
}

/// Notion APIクライアント
pub struct NotionClient {
    pub client: Client,
    pub config: Config,
    limiter: DefaultDirectRateLimiter,
}

/// Notion APIクライアント
/// インターフェースとなる関数は、newとquery_database, query_pageのみ
/// 実際のHTTPリクエスト処理は内部で行っており、具体的な処理はハードコードされている
impl NotionClient {
    /// 新しいNotionClientを生成する
    pub fn new(config: Config) -> Self {
        // 1秒あたり2リクエストのレートリミッターを設定
        // 実際には3リクエスト可能だが安全のため、2リクエストに設定
        let quota =
            Quota::per_second(NonZeroU32::new(2).unwrap()).allow_burst(NonZeroU32::new(2).unwrap());
        let limiter = governor::RateLimiter::direct(quota);
        Self {
            client: reqwest::Client::new(),
            config,
            limiter,
        }
    }

    /// リクエストの共通部分であるヘッダーを設定する
    fn request(&self, method: reqwest::Method, url: &str) -> Result<reqwest::RequestBuilder> {
        let mut auth_value =
            HeaderValue::from_str(&format!("Bearer {}", self.config.notion_token))?;
        auth_value.set_sensitive(true);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth_value);
        headers.insert("Notion-Version", HeaderValue::from_static("2022-06-28"));

        Ok(self.client.request(method, url).headers(headers))
    }

    /// GETリクエスト
    async fn http_get(&self, url: &str) -> Result<serde_json::Value> {
        self.limiter.until_ready().await;
        let resp = self
            .request(reqwest::Method::GET, url)?
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    /// POSTリクエスト
    async fn http_post<B: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<serde_json::Value> {
        self.limiter.until_ready().await;
        let resp = self
            .request(reqwest::Method::POST, url)?
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    /// paginationで分割されたレスポンスを取得するための共通関数
    async fn paginate_all<T, F, Fut>(&self, mut f: F) -> Result<Vec<T>>
    where
        F: FnMut(Option<String>) -> Fut,
        Fut: std::future::Future<Output = Result<NotionResponse<T>>>,
    {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let resp = f(cursor.clone()).await?;
            all.extend(resp.results);
            cursor = resp.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(all)
    }

    /// JSONからnext_cursorを抽出(has_more を元に判定)
    fn get_next_cursor(&self, json: &serde_json::Value) -> Option<String> {
        if json
            .get("has_more")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            json.get("next_cursor")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        } else {
            None
        }
    }

    /// データベース全件取得
    pub async fn fetch_database(&self) -> Result<Vec<PageInfo>> {
        self.paginate_all(|c| self.fetch_database_chunk(c)).await
    }

    /// チャンク毎にデータベース情報を取得
    async fn fetch_database_chunk(
        &self,
        start_cursor: Option<String>,
    ) -> Result<NotionResponse<PageInfo>> {
        let body = QueryBody {
            filter: QueryFilter {
                property: "ステータス",
                status: StatusEq { equals: "完了" },
            },
            start_cursor: start_cursor.as_deref(),
        };
        let url = format!(
            "https://api.notion.com/v1/databases/{}/query",
            self.config.database_id
        );
        let json = self.http_post(&url, &body).await?;

        let pages = extract_pages(&json)?;
        let next_cursor = self.get_next_cursor(&json);
        Ok(NotionResponse {
            results: pages,
            next_cursor,
        })
    }

    /// ページのブロック全件取得
    pub async fn fetch_page(&self, page: &PageInfo) -> Result<Vec<BlockInfo>> {
        self.paginate_all(|c| self.fetch_page_chunk(page, c)).await
    }

    /// チャンク毎にページ内ブロックを取得
    async fn fetch_page_chunk(
        &self,
        page: &PageInfo,
        start_cursor: Option<String>,
    ) -> Result<NotionResponse<BlockInfo>> {
        let mut url = Url::parse(&format!(
            "https://api.notion.com/v1/blocks/{}/children",
            page.id
        ))?;
        if let Some(c) = start_cursor {
            url.query_pairs_mut().append_pair("start_cursor", &c);
        }
        let json = self.http_get(url.as_str()).await?;
        let blocks = extract_blocks(&json)?;
        let next_cursor = self.get_next_cursor(&json);
        Ok(NotionResponse {
            results: blocks,
            next_cursor,
        })
    }
}
