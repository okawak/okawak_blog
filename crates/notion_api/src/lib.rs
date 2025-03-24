use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::fs;

/// Notion APIに必要な情報の構造体
#[derive(Deserialize)]
pub struct Config {
    notion_token: String,
    database_id: String,
}

/// 環境変数からシークレットトークンとデータベースIDを読み込む関数
pub fn load_config() -> Result<Config, Box<dyn Error>> {
    let notion_token = std::env::var("NOTION_TOKEN")?;
    let database_id = std::env::var("DATABASE_ID")?;
    Ok(Config {
        notion_token,
        database_id,
    })
}

/// ページネーションを管理する構造体
/// T: データベースクエリの場合、PageInfo構造体
///    ページに対するクエリの場合、未実装、仮にValue型
/// next_cursor: Noneの場合は最後のページ
struct Pagination<T> {
    contents: Vec<T>,
    next_cursor: Option<String>,
}

pub struct PageInfo {
    /// 子ページを取得するために必要なID
    id: String,

    /// ページのタイトル
    title: String,

    /// ページに付与されたタグ
    tags: Vec<String>,

    /// ページの作成日時
    created_time: String,

    /// ページの最終更新日時
    last_edited_time: String,

    /// ページのステータス (完了、進行中、未着手など)
    status: String,
    // 必要に応じて追加のフィールドを定義
}

impl PageInfo {
    pub async fn query_page(&self, client: &Client, config: &Config) -> Result<(), Box<dyn Error>> {
        let mut all_blocks = Vec::new();
        let block_data = self.query_page_chunk(client, config, None).await?;
        all_blocks.extend(block_data.contents);

        let mut next_cursor = block_data.next_cursor;
        while let Some(start_cursor) = next_cursor.as_ref() {
            let next_block_data = self
                .query_page_chunk(client, config, Some(start_cursor))
                .await?;
            all_blocks.extend(next_block_data.contents);
            next_cursor = next_block_data.next_cursor;
        }
        let output_str = serde_json::to_string_pretty(&all_blocks)?;

        // 以下は内容を解析し、Markdownに変換する処理を今後実装する
        // 仮にjson形式でファイルに保存する
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let path_str = format!("{}/dest/{}.json", manifest_dir, self.id);
        let path = std::path::Path::new(&path_str);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?; // ディレクトリを作成
        }
        fs::write(path, output_str)?;
        // ここまで
        Ok(())
    }

    async fn query_page_chunk(
        &self,
        client: &Client,
        config: &Config,
        next_cursor: Option<&String>,
    ) -> Result<Pagination<Value>, Box<dyn Error>> {
        let url = if let Some(start_cursor) = next_cursor {
            format!(
                "https://api.notion.com/v1/blocks/{}/children?page_size=100&start_cursor={}",
                self.id, start_cursor
            )
        } else {
            format!(
                "https://api.notion.com/v1/blocks/{}/children?page_size=100",
                self.id
            )
        };
        let res = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", config.notion_token))
            .header("Notion-Version", "2022-06-28")
            .send()
            .await?;

        let json: Value = res.json().await?;
        let next_cursor = if json.get("has_more").and_then(|v| v.as_bool()).unwrap() {
            json.get("next_cursor")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        let vec = vec![json];
        Ok(Pagination {
            contents: vec,
            next_cursor,
        })
    }
}
pub async fn query_database(
    client: &Client,
    config: &Config,
) -> Result<Vec<PageInfo>, Box<dyn Error>> {
    let mut all_pages = Vec::new();
    let page_data = query_database_chunk(client, config, None).await?;
    all_pages.extend(page_data.contents);

    let mut next_cursor = page_data.next_cursor;
    while let Some(start_cursor) = next_cursor.as_ref() {
        let next_page_data = query_database_chunk(client, config, Some(start_cursor)).await?;
        all_pages.extend(next_page_data.contents);
        next_cursor = next_page_data.next_cursor;
    }
    Ok(all_pages)
}

async fn query_database_chunk(
    client: &Client,
    config: &Config,
    next_cursor: Option<&String>,
) -> Result<Pagination<PageInfo>, Box<dyn Error>> {
    // 100件以上のデータの場合はnext_cursorを指定して続きから取得
    let body_str = format!(
        r#"{{ {}
              "filter": {{
                "property": "ステータス",
                "status": {{
                  "equals": "完了"
                }}
              }}
           }}"#,
        if let Some(start_cursor) = next_cursor {
            format!(r#", "start_cursor": "{}","#, start_cursor)
        } else {
            "".to_string()
        }
    );

    let res = client
        .post(format!(
            "https://api.notion.com/v1/databases/{}/query",
            config.database_id
        ))
        .header("Authorization", format!("Bearer {}", config.notion_token))
        .header("Notion-Version", "2022-06-28")
        .header("Content-Type", "application/json")
        .body(body_str)
        .send()
        .await?;
    let json: Value = res.json().await?;

    let next_cursor = if json.get("has_more").and_then(|v| v.as_bool()).unwrap() {
        json.get("next_cursor")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    let pages = extract_pages(&json)?;
    Ok(Pagination {
        contents: pages,
        next_cursor,
    })
}

fn extract_pages(json: &Value) -> Result<Vec<PageInfo>, Box<dyn Error>> {
    let results = json
        .get("results")
        .and_then(|v| v.as_array())
        .ok_or("No results array")?;
    // 予期しないデータ構造だった場合はパニックを起こすようになっている
    // (unwrapを使用)
    let pages = results
        .iter()
        .map(|page| {
            let id = page.get("id").and_then(|v| v.as_str()).unwrap().to_string();
            let properties = page.get("properties").unwrap();
            let title = properties
                .get("Name")
                .and_then(|v| v.get("title"))
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("plain_text"))
                .and_then(|v| v.as_str())
                .unwrap()
                .to_string();
            let tags = properties
                .get("タグ")
                .and_then(|v| v.get("multi_select"))
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|tag| {
                    tag.get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();
            let created_time = page
                .get("created_time")
                .and_then(|v| v.as_str())
                .unwrap()
                .to_string();
            let last_edited_time = page
                .get("last_edited_time")
                .and_then(|v| v.as_str())
                .unwrap()
                .to_string();
            let status = properties
                .get("ステータス")
                .and_then(|v| v.get("status"))
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .unwrap()
                .to_string();
            PageInfo {
                id,
                title,
                tags,
                created_time,
                last_edited_time,
                status,
            }
        })
        .collect();
    Ok(pages)
}
