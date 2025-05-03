use crate::error::Result;
use serde_json::Value;

pub fn process(value: &Value) -> Result<String> {
    Ok(format!("link to page: {value:?}\n\n").to_string())
}
