use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::fs;

#[derive(Deserialize)]
struct Config {
    notion_token: String,
    database_id: String,
}

// 設定ファイル(config.json)からシークレットトークンとデータベースIDを読み込む関数
fn load_config() -> Result<Config, Box<dyn Error>> {
    let config_str = fs::read_to_string("config.json")?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}

// Notion APIのデータベースクエリを実行する関数
async fn query_database(client: &Client, config: &Config) -> Result<Value, Box<dyn Error>> {
    let url = format!(
        "https://api.notion.com/v1/databases/{}/query",
        config.database_id
    );
    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.notion_token))
        .header("Notion-Version", "2022-06-28") // APIバージョンは適宜更新してください
        .header("Content-Type", "application/json")
        .body("{}") // フィルタやソート条件が必要な場合はここを変更
        .send()
        .await?;
    let json: Value = res.json().await?;
    Ok(json)
}

// 指定したブロック（ここではページ）の子ブロックを取得する関数
async fn get_child_blocks(
    client: &Client,
    token: &str,
    block_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let url = format!("https://api.notion.com/v1/blocks/{}/children", block_id);
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", "2022-06-28")
        .send()
        .await?;
    let json: Value = res.json().await?;
    Ok(json)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 設定ファイルからシークレットトークンとデータベースIDを読み込む
    let config = load_config()?;

    let client = Client::new();

    // データベースの内容を取得
    let mut db_result = query_database(&client, &config).await?;

    // 取得結果の各ページに対して、子ブロック（子ページ）の内容を取得し、結果に追加する
    if let Some(results) = db_result.get_mut("results").and_then(|v| v.as_array_mut()) {
        results.retain(|page| {
            // "properties" → "ステータス" → "select" → "name" が "完了" かどうかをチェック
            if let Some(status) = page
                .get("properties")
                .and_then(|props| props.get("ステータス"))
                .and_then(|prop| prop.get("status"))
                .and_then(|select| select.get("name"))
                .and_then(|name| name.as_str())
            {
                status == "完了"
            } else {
                false
            }
        });

        for page in results.iter_mut() {
            if let Some(page_id) = page.get("id").and_then(|v| v.as_str()) {
                // 子ブロック取得APIを呼び出す
                match get_child_blocks(&client, &config.notion_token, page_id).await {
                    Ok(child_blocks) => {
                        // ページオブジェクトに "child_blocks" フィールドを追加する
                        if let Some(obj) = page.as_object_mut() {
                            obj.insert("child_blocks".to_string(), child_blocks);
                        }
                    }
                    Err(e) => {
                        eprintln!("ページ {} の子ブロック取得に失敗しました: {}", page_id, e);
                    }
                }
            }
        }
    }

    // 結果をファイル (output.json) に保存する
    let output_str = serde_json::to_string_pretty(&db_result)?;
    fs::write("output.json", output_str)?;

    println!("結果を output.json に保存しました。");

    Ok(())
}
