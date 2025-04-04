//! Command argument processing prism
//!
//! This prism provides functionality for parsing and validating command line
//! arguments against JSON schemas, as well as generating help text for commands.

use std::sync::Arc;
use uuid::Uuid;

use uv_core::{
    Result, UVError, UVLink, UVPrism, PrismMultiplexer, UVPulse, UVSpectrum, 
    Wavefront, args_processor::ArgsProcessor
};
use serde_json::json;

/// Command prism for processing arguments
pub struct CommandPrism {
    spectrum: Option<UVSpectrum>,
}

impl CommandPrism {
    /// Create a new command prism
    pub fn new() -> Self {
        Self { spectrum: None }
    }
    
    /// Handle parse frequency
    fn handle_parse(&self, id: Uuid, wavefront: &Wavefront, link: &UVLink) -> Result<bool> {
        // Extract required parameters and parse schema if needed
        let schema_value = match wavefront.input.get("schema") {
            Some(s) => s,
            None => {
                link.emit_trap(id, Some(UVError::InvalidInput("Missing schema parameter".to_string())))?;
                return Err(UVError::InvalidInput("Missing schema parameter".to_string()));
            }
        };
        
        // Parse the schema if it's a string
        let schema = if let Some(schema_str) = schema_value.as_str() {
            // It's a string, we need to parse it
            match serde_json::from_str::<serde_json::Value>(schema_str) {
                Ok(parsed) => parsed,
                Err(e) => {
                    let error_msg = format!("Failed to parse schema JSON string: {}", e);
                    println!("DEBUG: Error parsing schema: {}", error_msg);
                    link.emit_trap(id, Some(UVError::InvalidInput(error_msg.clone())))?;
                    return Err(UVError::InvalidInput(error_msg));
                }
            }
        } else {
            // It's already a JSON value, use it as-is
            schema_value.clone()
        };
        
        let args = match wavefront.input.get("args") {
            Some(a) => a.as_array()
                .ok_or_else(|| UVError::InvalidInput("args must be an array".to_string()))?
                .iter()
                .filter_map(|a| a.as_str().map(ToString::to_string))
                .collect::<Vec<String>>(),
            None => {
                link.emit_trap(id, Some(UVError::InvalidInput("Missing args parameter".to_string())))?;
                return Err(UVError::InvalidInput("Missing args parameter".to_string()));
            }
        };
        
        // Create schema definition from the input
        let schema_def = uv_core::SchemaDefinition {
            schema: schema.clone(),
            required: schema.get("required")
                .and_then(|r| r.as_array())
                .map_or_else(Vec::new, |arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(ToString::to_string))
                        .collect()
                }),
        };
        
        // Process the arguments
        let processor = ArgsProcessor::new(schema_def);
        match processor.process_args(&args) {
            Ok(result) => {
                link.emit_photon(id, result)?;
                link.emit_trap(id, None)?;
                Ok(true)
            },
            Err(e) => {
                let error = UVError::InvalidInput(format!("Error processing arguments: {}", e));
                link.emit_trap(id, Some(error.clone()))?;
                Err(error)
            }
        }
    }
    
    /// Handle validate frequency
    fn handle_validate(&self, id: Uuid, wavefront: &Wavefront, link: &UVLink) -> Result<bool> {
        // Extract required parameters
        let schema = match wavefront.input.get("schema") {
            Some(s) => s,
            None => {
                link.emit_trap(id, Some(UVError::InvalidInput("Missing schema parameter".to_string())))?;
                return Err(UVError::InvalidInput("Missing schema parameter".to_string()));
            }
        };
        
        let input = match wavefront.input.get("input") {
            Some(i) => i,
            None => {
                link.emit_trap(id, Some(UVError::InvalidInput("Missing input parameter".to_string())))?;
                return Err(UVError::InvalidInput("Missing input parameter".to_string()));
            }
        };
        
        // Create schema definition from the input
        let schema_def = uv_core::SchemaDefinition {
            schema: schema.clone(),
            required: schema.get("required")
                .and_then(|r| r.as_array())
                .map_or_else(Vec::new, |arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(ToString::to_string))
                        .collect()
                }),
        };
        
        // Perform validation (simplified for now)
        // Just check if all required fields are present
        let mut errors = Vec::new();
        for field in &schema_def.required {
            if !input.as_object().unwrap_or(&serde_json::Map::new()).contains_key(field) {
                errors.push(format!("Missing required field: {}", field));
            }
        }
        
        let valid = errors.is_empty();
        
        // Return validation result
        let result = json!({
            "valid": valid,
            "errors": errors,
        });
        
        link.emit_photon(id, result)?;
        link.emit_trap(id, None)?;
        Ok(true)
    }
    
    /// Handle describe frequency
    fn handle_describe(&self, id: Uuid, wavefront: &Wavefront, link: &UVLink) -> Result<bool> {
        // Extract required parameters and parse schema if needed
        let schema_value = match wavefront.input.get("schema") {
            Some(s) => s,
            None => {
                link.emit_trap(id, Some(UVError::InvalidInput("Missing schema parameter".to_string())))?;
                return Err(UVError::InvalidInput("Missing schema parameter".to_string()));
            }
        };
        
        // Parse the schema if it's a string
        let schema = if let Some(schema_str) = schema_value.as_str() {
            // It's a string, we need to parse it
            match serde_json::from_str::<serde_json::Value>(schema_str) {
                Ok(parsed) => parsed,
                Err(e) => {
                    let error_msg = format!("Failed to parse schema JSON string: {}", e);
                    println!("DEBUG: Error parsing schema: {}", error_msg);
                    link.emit_trap(id, Some(UVError::InvalidInput(error_msg.clone())))?;
                    return Err(UVError::InvalidInput(error_msg));
                }
            }
        } else {
            // It's already a JSON value, use it as-is
            schema_value.clone()
        };
        
        let command_name = wavefront.input.get("command_name")
            .and_then(|c| c.as_str())
            .unwrap_or("command");
        
        // Extract description if available
        let description = schema.get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        
        // Build options list from properties
        let mut options = Vec::new();
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            for (name, prop) in properties {
                let option = json!({
                    "name": name,
                    "description": prop.get("description").and_then(|d| d.as_str()).unwrap_or(""),
                    "type": prop.get("type").and_then(|t| t.as_str()).unwrap_or("string"),
                    "required": schema.get("required")
                        .and_then(|r| r.as_array())
                        .map_or(false, |arr| arr.iter().any(|s| s.as_str() == Some(name))),
                });
                
                options.push(option);
            }
        }
        
        // Generate basic usage string
        let mut usage = format!("{}", command_name);
        let required_options = options.iter()
            .filter(|o| o.get("required").and_then(|r| r.as_bool()).unwrap_or(false))
            .map(|o| format!("<{}>", o.get("name").and_then(|n| n.as_str()).unwrap_or("")))
            .collect::<Vec<_>>()
            .join(" ");
        
        if !required_options.is_empty() {
            usage = format!("{} {}", usage, required_options);
        }
        
        usage = format!("{} [options]", usage);
        
        // Add debug logging
        println!("DEBUG: Schema: {:?}", schema);
        println!("DEBUG: Properties count: {}", schema.get("properties")
            .and_then(|p| p.as_object())
            .map_or(0, |p| p.len()));
        println!("DEBUG: Options count: {}", options.len());
        println!("DEBUG: Options: {:?}", options);
        
        // Return help information with simpler format for debugging
        let result = json!({
            "usage": usage,
            "description": description,
            "options": options,
            "debug_info": {
                "has_properties": schema.get("properties").is_some(),
                "options_count": options.len()
            }
        });
        
        // Print the result for debugging
        println!("DEBUG: Result: {}", serde_json::to_string_pretty(&result).unwrap_or_default());
        
        link.emit_photon(id, result)?;
        link.emit_trap(id, None)?;
        Ok(true)
    }
}

impl UVPrism for CommandPrism {
    fn init_spectrum(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn init_multiplexer(&mut self, _multiplexer: Arc<PrismMultiplexer>) -> Result<()> {
        // Command prism doesn't need the multiplexer
        Ok(())
    }
    
    fn spectrum(&self) -> &UVSpectrum {
        self.spectrum.as_ref().expect("Prism not initialized")
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "parse" => self.handle_parse(id, wavefront, link),
                "validate" => self.handle_validate(id, wavefront, link),
                "describe" => self.handle_describe(id, wavefront, link),
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}

#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(CommandPrism::new())
}
