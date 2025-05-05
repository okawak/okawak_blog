use crate::error::Result;
use serde_json::Value;

pub fn process(value: &Value) -> Result<String> {
    let url = value.get("url").unwrap().to_string();
    let url = url.trim_matches('"');
    Ok(format!("<div class=\"notion-bookmark\" data-url=\"{url}\"></div>\n\n").to_string())
}
