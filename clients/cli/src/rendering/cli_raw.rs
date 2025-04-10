use anyhow::Result;
use serde_json::Value;

pub fn render_raw_array(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);

    Ok(())
}   

pub fn render_raw_object(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);

    Ok(())
}   