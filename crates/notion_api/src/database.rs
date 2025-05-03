use crate::error::{NotionError, Result};
use crate::models::PageInfo;
use serde_json::Value;

/// Notion APIのデータベースレスポンスからPageInfoのベクターを抽出する
pub fn extract_pages(json: &Value) -> Result<Vec<PageInfo>> {
    let results = json
        .get("results")
        .and_then(|v| v.as_array())
        .ok_or(NotionError::DataError(format!(
            "No results array: {json:?}"
        )))?;
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
            let category = properties
                .get("カテゴリー")
                .and_then(|v| v.get("select"))
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let group = properties
                .get("グループ")
                .and_then(|v| v.get("select"))
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let priority_level = properties
                .get("優先度")
                .and_then(|v| v.get("number"))
                .and_then(|v| v.as_i64())
                .map(|v| v as i32)
                .unwrap_or(0);
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
            let summary = properties
                .get("要約")
                .and_then(|v| v.get("rich_text"))
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("plain_text"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
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
                category,
                group,
                priority_level,
                tags,
                summary,
                created_time,
                last_edited_time,
                status,
            }
        })
        .collect();
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_pages() {
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
                        "グループ": {
                            "select": {
                                "name": "基礎",
                             }
                        },
                        "ステータス": {
                            "status": {
                                "name": "完了"
                            }
                        },
                        "要約": {
                            "rich_text": [
                                {
                                    "plain_text": "likelihoodの概念と、それを使った最尤推定の方法のイメージについてのページです。",
                                }
                            ]
                        },
                        "優先度": { "number": 1 },
                        "カテゴリー": {
                            "select": {
                                "name": "テスト"
                            }
                        }
                    }
                }
            ],
            "has_more": false
        });
        let pages = extract_pages(&sample_json).unwrap();
        assert_eq!(pages.len(), 1);
        let page = &pages[0];
        assert_eq!(page.id, "page_1");
        assert_eq!(page.title, "テストページ");
        assert_eq!(page.category, "テスト");
        assert_eq!(page.tags, vec!["タグ1".to_string(), "タグ2".to_string()]);
    }
}
