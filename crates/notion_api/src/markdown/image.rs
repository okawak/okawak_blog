use crate::error::Result;
use serde_json::Value;

pub fn process(value: &Value) -> Result<String> {
    let url = value
        .get("file")
        .and_then(|v| v.get("url"))
        .unwrap()
        .to_string();
    let url = url.trim_matches('"');
    // Notionの画像URLは一時的なものなので、ダウンロードして保存する処理を書きたい
    Ok(format!("![image]({url})\n\n"))
}
