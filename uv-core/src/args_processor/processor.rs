//! Processor for schema-based argument handling.
//!
//! This module provides the main ArgsProcessor that combines argument parsing with
//! schema-based processing to generate structured command input.

use crate::spectrum::{SchemaDefinition, UVSpectrum};
use crate::UVError;
use super::parser::{parse_args, ParsedArgs};
use serde_json::{Value, json};
use thiserror::Error;
use std::str::FromStr;

/// Errors that can occur during argument processing
#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("Schema not found for frequency: {0}")]
    SchemaNotFound(String),
    
    #[error("Invalid parameter type: {0}")]
    InvalidType(String),
    
    #[error("Invalid value for parameter {0}: {1}")]
    InvalidValue(String, String),
    
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),
    
    #[error("Error processing arguments: {0}")]
    Other(String),
}

impl From<UVError> for ProcessorError {
    fn from(err: UVError) -> Self {
        ProcessorError::Other(err.to_string())
    }
}

/// Processor for command line arguments based on a JSON schema
pub struct ArgsProcessor {
    /// Schema definition for the command
    schema: SchemaDefinition,
}

impl ArgsProcessor {
    /// Create a new processor from a spectrum and frequency
    pub fn from_spectrum(spectrum: &UVSpectrum, frequency: &str) -> Result<Self, ProcessorError> {
        let schema = spectrum.get_input_schema(frequency)
            .ok_or_else(|| ProcessorError::SchemaNotFound(frequency.to_string()))?
            .clone();
        
        Ok(Self::new(schema))
    }
    
    /// Create a new processor with the given schema
    pub fn new(schema: SchemaDefinition) -> Self {
        Self { schema }
    }
    
    /// Process command line arguments into a structured value
    pub fn process_args(&self, args: &[String]) -> Result<Value, ProcessorError> {
        let parsed = parse_args(args);
        self.process_parsed(parsed)
    }
    
    /// Process already-parsed arguments
    pub fn process_parsed(&self, parsed: ParsedArgs) -> Result<Value, ProcessorError> {
        let mut result = json!({});
        let obj = result.as_object_mut().unwrap();
        
        // Process named arguments
        for (name, values) in parsed.named_args {
            if self.is_array_param(&name) {
                // For array types, include all values as an array
                obj.insert(name.clone(), json!(values));
            } else {
                // For non-array types, use the first value with appropriate type conversion
                if let Some(first) = values.first() {
                    let value = self.convert_value_for_param(&name, first)?;
                    obj.insert(name.clone(), value);
                }
            }
        }
        
        // Process boolean flags
        for (name, value) in parsed.flags {
            if self.is_boolean_param(&name) {
                obj.insert(name.clone(), json!(value));
            }
        }
        
        // Process positional arguments based on required parameters
        let required_params = self.get_required_string_params();
        let mut pos_idx = 0;
        
        for param in required_params {
            // Skip if already set by named argument
            if !obj.contains_key(&param) && pos_idx < parsed.positional_args.len() {
                let value = self.convert_value_for_param(&param, &parsed.positional_args[pos_idx])?;
                obj.insert(param.clone(), value);
                pos_idx += 1;
            }
        }
        
        // Add any default values for parameters not specified
        self.apply_defaults(&mut result)?;
        
        // Validate against required parameters
        self.validate_required(&result)?;
        
        Ok(result)
    }
    
    /// Get the required string parameters from the schema
    fn get_required_string_params(&self) -> Vec<String> {
        let mut required = Vec::new();
        
        if let Some(required_array) = self.schema.schema.get("required") {
            if let Some(required_params) = required_array.as_array() {
                for param in required_params {
                    if let Some(param_name) = param.as_str() {
                        if let Some(param_type) = self.get_param_type(param_name) {
                            if param_type == "string" {
                                required.push(param_name.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        required
    }
    
    /// Check if a parameter is a boolean type
    fn is_boolean_param(&self, name: &str) -> bool {
        self.get_param_type(name) == Some("boolean")
    }
    
    /// Check if a parameter is an array type
    fn is_array_param(&self, name: &str) -> bool {
        // Check if this parameter is an array type
        if self.get_param_type(name) == Some("array") {
            return true;
        }
        
        // Check for singular/plural variations as in the original CLI
        // If name doesn't end in 's', check if plural form is an array
        if !name.ends_with('s') {
            let plural = format!("{}s", name);
            if self.get_param_type(&plural) == Some("array") {
                return true;
            }
        }
        // If name ends in 's', check if singular form is an array
        else if name.len() > 1 {
            let singular = &name[..name.len()-1];
            if self.get_param_type(singular) == Some("array") {
                return true;
            }
        }
        
        false
    }
    
    /// Get the canonical name for an array parameter (handling singular/plural)
    #[allow(dead_code)]
    fn get_array_param_name(&self, name: &str) -> String {
        // If name doesn't end in 's' but plural form is an array
        if !name.ends_with('s') {
            let plural = format!("{}s", name);
            if self.get_param_type(&plural) == Some("array") {
                return plural;
            }
        }
        // If name ends in 's' but singular form is an array
        else if name.len() > 1 {
            let singular = &name[..name.len()-1];
            if self.get_param_type(singular) == Some("array") {
                return singular.to_string();
            }
        }
        
        // Default to original name
        name.to_string()
    }
    
    /// Get the type of a parameter from the schema
    fn get_param_type(&self, param_name: &str) -> Option<&str> {
        // Check for direct property
        let properties = self.schema.schema.get("properties")?;
        let param = properties.get(param_name)?;
        param.get("type")?.as_str()
    }
    
    /// Convert a string value to the appropriate JSON value based on parameter type
    fn convert_value_for_param(&self, param_name: &str, value: &str) -> Result<Value, ProcessorError> {
        let param_type = self.get_param_type(param_name);
        
        match param_type {
            Some("string") => Ok(json!(value)),
            
            Some("number") | Some("integer") => {
                if let Ok(num) = f64::from_str(value) {
                    Ok(json!(num))
                } else {
                    Err(ProcessorError::InvalidValue(
                        param_name.to_string(),
                        format!("Expected number, got: {}", value)
                    ))
                }
            },
            
            Some("boolean") => {
                match value.to_lowercase().as_str() {
                    "true" | "yes" | "1" | "on" => Ok(json!(true)),
                    "false" | "no" | "0" | "off" => Ok(json!(false)),
                    _ => Err(ProcessorError::InvalidValue(
                        param_name.to_string(),
                        format!("Expected boolean, got: {}", value)
                    )),
                }
            },
            
            _ => Ok(json!(value)), // Default to string for unknown types
        }
    }
    
    /// Apply default values from the schema
    fn apply_defaults(&self, result: &mut Value) -> Result<(), ProcessorError> {
        let obj = result.as_object_mut().unwrap();
        
        if let Some(properties) = self.schema.schema.get("properties") {
            if let Some(props) = properties.as_object() {
                for (name, schema) in props {
                    // Skip if already set
                    if obj.contains_key(name) {
                        continue;
                    }
                    
                    // Apply default if present
                    if let Some(default) = schema.get("default") {
                        obj.insert(name.clone(), default.clone());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate that all required parameters are present
    fn validate_required(&self, value: &Value) -> Result<(), ProcessorError> {
        let obj = value.as_object().unwrap();
        
        // Check explicit 'required' array in schema root
        if let Some(required_array) = self.schema.schema.get("required") {
            if let Some(required_params) = required_array.as_array() {
                for param in required_params {
                    if let Some(param_name) = param.as_str() {
                        if !obj.contains_key(param_name) {
                            return Err(ProcessorError::MissingParameter(param_name.to_string()));
                        }
                    }
                }
            }
        }
        
        // Also check the 'required' field on the SchemaDefinition (common in Ultraviolet)
        for param_name in &self.schema.required {
            if !obj.contains_key(param_name) {
                return Err(ProcessorError::MissingParameter(param_name.clone()));
            }
        }
        
        Ok(())
    }
}