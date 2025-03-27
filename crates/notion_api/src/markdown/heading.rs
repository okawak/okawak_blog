use serde_json::Value;
use std::error::Error;

pub fn process(value: &Value, size: u8) -> Result<String, Box<dyn Error>> {
    Ok(format!("heading {}\n\n", size).to_string())
}
