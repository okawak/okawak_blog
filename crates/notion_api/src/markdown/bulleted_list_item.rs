use serde_json::Value;
use std::error::Error;

pub fn process(value: &Value) -> Result<String, Box<dyn Error>> {
    let mut list_item = String::new();
    list_item.push_str("- ");
    for element in value.get("rich_text").unwrap().as_array().unwrap() {
        // もちろん他にも色々な要素があるが、今のところはtextだけを考える
        if let Some(text) = element.get("text") {
            if let Some(content) = text.get("content") {
                list_item.push_str(content.to_string().trim_matches('"'));
            }
        }
    }
    Ok(format!("{}\n\n", list_item).to_string())
}
