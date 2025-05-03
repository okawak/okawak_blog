use crate::error::Result;
use serde_json::Value;

pub fn process(value: &Value) -> Result<String> {
    let expression = value.get("expression").unwrap().to_string();
    let mut content = expression.trim_matches('"').to_string();
    content = content.replace("\\\\", "\\");
    content = format!("$${content}$$");
    Ok(format!("{content}\n\n").to_string())
}
