use crate::config::Config;
use crate::database::extract_pages;
use crate::markdown::convert_page_to_markdown;
use crate::models::{BlockInfo, PageInfo};
use reqwest::Client;
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::path::Path;

/// ページ分割されたデータを処理する構造体
/// 内部処理のみに用いるのでpubではない
struct Pagination<T> {
    contents: Vec<T>,
    next_cursor: Option<String>,
}

pub struct NotionClient {
    pub client: Client,
    pub config: Config,
}

impl NotionClient {
    pub fn new(config: Config) -> Self {
        NotionClient {
            client: Client::new(),
            config,
        }
    }

    async fn get(&self, url: &str) -> Result<Value, Box<dyn Error>> {
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

    async fn post(&self, url: &str, body: &str) -> Result<Value, Box<dyn Error>> {
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

    pub async fn query_database(&self) -> Result<Vec<PageInfo>, Box<dyn Error>> {
        let mut all_pages = Vec::new();
        let mut next_cursor: Option<String> = None;
        loop {
            let pagination: Pagination<PageInfo> =
                self.query_database_chunk(next_cursor.as_ref()).await?;
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
    ) -> Result<Pagination<PageInfo>, Box<dyn Error>> {
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
        let json_response = self.post(&url, &body_str).await?;
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
    pub async fn query_page(&self, page: &PageInfo) -> Result<Vec<BlockInfo>, Box<dyn Error>> {
        let mut all_blocks = Vec::new();
        let mut next_cursor: Option<String> = None;
        loop {
            let pagination: Pagination<Value> =
                self.query_page_chunk(page, next_cursor.as_ref()).await?;
            all_blocks.extend(pagination.contents);
            if let Some(cursor) = pagination.next_cursor {
                next_cursor = Some(cursor);
            } else {
                break;
            }
        }
        // Markdownに変換する処理を呼び出し
        let markdown = convert_page_to_markdown(page, &all_blocks);

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let file_path = format!("{}/dest/{}.md", manifest_dir, page.id);
        if let Some(parent) = Path::new(&file_path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, markdown)?;
        Ok(())
    }

    async fn query_page_chunk(
        &self,
        page: &PageInfo,
        next_cursor: Option<&String>,
    ) -> Result<Pagination<BlockInfo>, Box<dyn Error>> {
        let url = if let Some(cursor) = next_cursor {
            format!(
                "https://api.notion.com/v1/blocks/{}/children?page_size=100&start_cursor={}",
                page.id, cursor
            )
        } else {
            format!(
                "https://api.notion.com/v1/blocks/{}/children?page_size=100",
                page.id
            )
        };
        let json_response = self.get(&url).await?;
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

        Ok(Pagination {
            contents: vec![json_response],
            next_cursor,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pages_integration() {
        // 簡単なJSONサンプルでextract_pagesの動作確認（database.rs内のテストと同様）
        let sample_json = json!({
            "results": [
                {
                    "id": "page_1",
                    "created_time": "2023-03-01T00:00:00.000Z",
                    "last_edited_time": "2023-03-02T00:00:00.000Z",
                    "properties": {
                        "Name": {
                            "title": [
                                { "plain_text": "テストページ" }
                            ]
                        },
                        "タグ": {
                            "multi_select": [
                                {"name": "タグ1"},
                                {"name": "タグ2"}
                            ]
                        },
                        "ステータス": {
                            "status": {
                                "name": "完了"
                            }
                        }
                    }
                }
            ],
            "has_more": false
        });
        let pages = crate::database::extract_pages(&sample_json).unwrap();
        assert_eq!(pages.len(), 1);
    }
}
