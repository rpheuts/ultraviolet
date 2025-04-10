use anyhow::Result;
use serde_json::Value;

pub fn render_csv_array(value: &Value) -> Result<()> {
    if let Some(array) = value.as_array() {
        for row in array {
            if let Some(row_object) = row.as_object() {
                let line = row_object
                    .values()
                    .map(|v| v.as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join(",");
                
                println!("{}", line);
            }
        }
    }

    Ok(())
}   

pub fn render_csv_object(value: &Value) -> Result<()> {
    if let Some(row_object) = value.as_object() {
        let line = row_object
            .values()
            .map(|v| v.as_str().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(",");
        
        println!("{}", line);
    }

    Ok(())
}   