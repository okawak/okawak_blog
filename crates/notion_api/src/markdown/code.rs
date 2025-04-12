use serde_json::Value;
use std::error::Error;

pub fn process(value: &Value) -> Result<String, Box<dyn Error>> {
    Ok(format!("code: {:?}\n\n", value).to_string())
}
