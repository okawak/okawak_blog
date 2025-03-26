use crate::models::PageInfo;
use serde_json::Value;
use std::error::Error;

pub fn extract_blocks(json: &Value) -> Result<&Vec<Value>, Box<dyn Error>> {
    let results = json
        .get("results")
        .and_then(|v| v.as_array())
        .ok_or("No results array")?;
    Ok(results)
}

/// ページの子ブロックのJSONデータをMarkdown形式の文字列に変換する
pub fn convert_page_to_markdown(page_info: &PageInfo, blocks: &[Value]) -> String {
    let mut markdown = String::new();

    // ここでは例として、ブロックの種類が "paragraph" の場合のみテキストを抽出する簡単な実装
    for block in blocks {
        if let Some(block_type) = block.get("type").and_then(|v| v.as_str()) {
            match block_type {
                "paragraph" => {
                    if let Some(text_array) = block
                        .get("paragraph")
                        .and_then(|p| p.get("text"))
                        .and_then(|t| t.as_array())
                    {
                        for text_item in text_array {
                            if let Some(plain_text) =
                                text_item.get("plain_text").and_then(|v| v.as_str())
                            {
                                markdown.push_str(plain_text);
                            }
                        }
                        markdown.push_str("\n\n");
                    }
                }
                // 他のブロックタイプについての変換処理もここで追加可能
                _ => {
                    // 例: 見出しなど
                    markdown.push_str(&format!("**{}**\n\n", block_type));
                }
            }
        }
    }
    markdown
}

//#[cfg(test)]
//mod tests {
//    use super::*;
//    use serde_json::json;
//
//    #[test]
//    fn test_convert_page_to_markdown() {
//        let blocks = vec![
//            json!({
//                "type": "paragraph",
//                "paragraph": {
//                    "text": [
//                        { "plain_text": "これはテストです。" }
//                    ]
//                }
//            }),
//            json!({
//                "type": "paragraph",
//                "paragraph": {
//                    "text": [
//                        { "plain_text": "Markdownへの変換例。" }
//                    ]
//                }
//            }),
//        ];
//
//        let markdown = convert_page_to_markdown(&blocks);
//        assert!(markdown.contains("これはテストです。"));
//        assert!(markdown.contains("Markdownへの変換例。"));
//    }
//}
