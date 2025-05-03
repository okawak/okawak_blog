use crate::error::Result;
use serde_json::Value;

pub fn process(value: &Value, size: u8) -> Result<String> {
    let mut title = String::new();
    for _ in 0..size {
        title.push('#');
    }
    title.push(' ');

    for element in value.get("rich_text").unwrap().as_array().unwrap() {
        // もちろん他にも色々な要素があるが、今のところはtextだけを考える
        if let Some(text) = element.get("text") {
            if let Some(content) = text.get("content") {
                title.push_str(content.to_string().trim_matches('"'));
            }
        }
    }
    Ok(format!("{title}\n\n").to_string())
}
