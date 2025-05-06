use crate::error::Result;
use serde_json::Value;

pub fn process(_value: &Value) -> Result<String> {
    Ok(format!("---\n\n"))
}
