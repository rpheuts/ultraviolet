use anyhow::Result;
use serde_json::Value;
use uv_core::UVSchemaDefinition;

pub fn render_cli_stream(_stream_type: &String, value: &Value, _schema: &UVSchemaDefinition) -> Result<()> {
    if let Some(items) = value.as_object() {
        if let Some(text) = items.values().next() {
            print!("{}", text.as_str().unwrap_or(""));
            return Ok(())
        }
    }

    print!("{}", serde_json::to_string_pretty(value)?);

    Ok(())
}