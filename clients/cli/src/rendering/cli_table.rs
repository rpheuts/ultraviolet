use anyhow::{anyhow, Result};
use serde_json::Value;
use uv_core::UVSchemaDefinition;
use tabled::{builder::Builder,settings::{object::Rows, themes::Colorization, Color, Style}};

pub fn render_cli_table(value: &Value, _schema: &UVSchemaDefinition) -> Result<()> {
    let values = match value.as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            println!("No records");
            return Ok(());
        }
    };

    // Assume all objects have the same keys; extract header from the first row
    let headers: Vec<String> = match values[0].as_object() {
        Some(obj) => obj.keys().map(|v| v.to_uppercase()).collect(),
        None => {
            println!("Invalid row format");
            return Ok(());
        }
    };

    let mut builder = Builder::new();
    builder.push_record(headers);

    for value in values {
        builder.push_record(render_table_row(value)?);
    }

    let mut table = builder.build();

    println!("{}", table
    .with(Colorization::exact([Color::FG_CYAN], Rows::first()))
    .with(Style::rounded()));

    Ok(())
}

fn render_table_row(row_object: &Value) -> Result<Vec<String>> {
    let values: Vec<String> = match row_object.as_object() {
        Some(obj) => obj.values().map(|v| v.as_str().unwrap().to_string()).collect(),
        None => {
            return Err(anyhow!("Unable to parse table row object"));
        }
    };

    Ok(values
        .iter()
        .map(|v| render_row_value(v))
        .collect())
}

fn render_row_value(value: &String) -> String {
    if value.starts_with("http") {
        return format!("\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\", value, "link")
    }

    value.clone()
}