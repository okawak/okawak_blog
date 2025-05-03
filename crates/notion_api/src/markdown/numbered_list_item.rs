use crate::error::Result;
use serde_json::Value;

pub fn process(value: &Value) -> Result<String> {
    Ok(format!("numbered list item: {value:?}\n\n").to_string())
}
