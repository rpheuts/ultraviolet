use anyhow::Result;
use serde_json::Value;
use uv_core::UVSchemaDefinition;

pub fn preprocess(input: Value, schema: &UVSchemaDefinition) -> Result<Value> {
    if let Some(default) = input.get("default") {
        if schema.required.len() == 1 {
            let mut processed = input
                .as_object()
                .unwrap()
                .clone();

            processed.insert(schema.required[0].clone(), default.clone());

            return Ok(serde_json::json!(processed));
        }
    }

    Ok(input)
}