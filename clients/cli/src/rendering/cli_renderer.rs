use anyhow::Result;
use serde_json::Value;
use uv_core::UVSchemaDefinition;

use super::{cli_card::render_cli_card, cli_stream::render_cli_stream, cli_table::render_cli_table};

pub fn render_array(value: &Value, schema: &UVSchemaDefinition) -> Result<()> {
    schema.validate(&value)?;

    render_cli_table(&value, schema)
}

pub fn render_object(value: &Value, schema: &UVSchemaDefinition) -> Result<()> {
    schema.validate(&value)?;

    render_cli_card(&value, schema)
}

pub fn render_stream(stream_type: &String, value: &Value, schema: &UVSchemaDefinition) -> Result<()> {
    schema.validate(value)?;

    render_cli_stream(stream_type, value, schema)
}