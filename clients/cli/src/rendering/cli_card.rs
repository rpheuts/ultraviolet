use anyhow::Result;
use serde_json::Value;
use uv_core::UVSchemaDefinition;
use tabled::{
    builder::Builder, settings::{Padding, Style}
};

pub fn render_cli_card(value: &Value, _schema: &UVSchemaDefinition) -> Result<()> {
    if let Some(obj) = value.as_object() {
        // Create a builder for our table
        let mut builder = Builder::new();
        
        // Add each key-value pair as a row
        for (key, val) in obj {
            let formatted_key = format!("\x1b[1;36m{}\x1b[0m:", key); // Bold cyan key with colon
            let formatted_value = format_value(val)?;
            builder.push_record(vec![formatted_key, formatted_value]);
        }
        
        // Build the table and apply styling
        let mut table = builder.build();

         let style = Style::rounded()
         .remove_horizontals()
         .remove_vertical();
        
        // Apply styling to create a card-like appearance
        println!("{}", table
            .with(style)
            .with(Padding::new(1, 2, 0, 0)) // Left padding of 1, right padding of 2
        );
    } else {
        // If not an object, fall back to pretty JSON
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    
    Ok(())
}

/// Helper function to format different value types
fn format_value(val: &Value) -> Result<String> {
    match val {
        // For complex types, pretty-print with indentation
        Value::Object(_) | Value::Array(_) => {
            let pretty = serde_json::to_string_pretty(val)?;
            Ok(pretty)
        },
        // Handle string values (remove extra quotes)
        Value::String(s) => Ok(s.clone()),
        // For other simple types, convert directly
        _ => Ok(val.to_string()),
    }
}
