use crate::error::Result;
use serde_json::Value;

pub fn process(value: &Value) -> Result<String> {
    Ok(format!("code: {value:?}\n\n").to_string())
}
