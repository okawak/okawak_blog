use serde_json::Value;
use std::error::Error;

pub fn process(value: &Value) -> Result<String, Box<dyn Error>> {
    let expression = value.get("expression").unwrap().to_string();
    let mut content = expression.trim_matches('"').to_string();
    content = content.replace("\\\\", "\\");
    content = format!("$$ {} $$", content);
    Ok(format!("{}\n\n", content).to_string())
}
