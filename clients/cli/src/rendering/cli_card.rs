use anyhow::Result;
use serde_json::Value;
use uv_core::UVSchemaDefinition;

pub fn render_cli_card(value: &Value, _schema: &UVSchemaDefinition) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);

    Ok(())
}