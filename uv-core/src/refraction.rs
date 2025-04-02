//! Refraction types for the Ultraviolet system.
//!
//! Refractions allow prisms to declare dependencies on other prisms and define
//! how data flows between them. They enable prisms to be self-contained while
//! still leveraging functionality from other prisms.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::error::{UVError, Result};

/// Definition of a dependency on another prism.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refraction {
    /// Local name for the refraction
    pub name: String,
    
    /// Target prism in namespace:name format
    pub target: String,
    
    /// Frequency to call on the target prism
    pub frequency: String,
    
    /// Input mapping (from refraction to target)
    #[serde(default)]
    pub transpose: HashMap<String, String>,
    
    /// Output mapping (from target to refraction)
    #[serde(default)]
    pub reflection: HashMap<String, String>,
}

impl Refraction {
    /// Parse the target into namespace and name.
    pub fn parse_target(&self) -> Result<(String, String)> {
        let parts: Vec<&str> = self.target.split(':').collect();
        if parts.len() != 2 {
            return Err(UVError::InvalidInput(format!("Invalid target format: {}", self.target)));
        }
        
        Ok((parts[0].to_string(), parts[1].to_string()))
    }
}

/// Property mapper for handling data flow between prisms.
#[derive(Debug, Clone)]
pub struct PropertyMapper {
    /// Mapping rules from source to target properties
    pub mapping_rules: HashMap<String, String>,
}

impl PropertyMapper {
    /// Create a new PropertyMapper with the given mapping rules.
    pub fn new(mapping_rules: HashMap<String, String>) -> Self {
        Self { mapping_rules }
    }
    
    /// Apply transpose mapping from source to target.
    ///
    /// This function maps properties from the source object to the target object
    /// according to the mapping rules.
    pub fn apply_transpose(&self, source: &Value) -> Result<Value> {
        // Create a new empty object for the target
        let mut target = serde_json::Map::new();
        
        // Apply each mapping rule
        for (target_path, source_path) in &self.mapping_rules {
            // Extract the value from the source using the source path
            let source_value = self.extract_value(source, source_path)?;
            
            // Insert the value into the target using the target path
            self.insert_value(&mut target, target_path, source_value)?;
        }
        
        Ok(Value::Object(target))
    }
    
    /// Apply reflection mapping from source to target.
    ///
    /// This function maps properties from the source object to the target object
    /// according to the mapping rules, but with source and target paths swapped.
    pub fn apply_reflection(&self, source: &Value) -> Result<Value> {
        // Create a new empty object for the target
        let mut target = serde_json::Map::new();
        
        // Apply each mapping rule (with source and target paths swapped)
        for (source_path, target_path) in &self.mapping_rules {
            // Extract the value from the source using the source path
            let source_value = self.extract_value(source, source_path)?;
            
            // Insert the value into the target using the target path
            self.insert_value(&mut target, target_path, source_value)?;
        }
        
        Ok(Value::Object(target))
    }
    
    /// Extract a value from an object using a path expression.
    ///
    /// The path is a dot-separated list of property names, e.g. "user.address.city".
    fn extract_value(&self, source: &Value, path: &str) -> Result<Value> {
        // Split the path into components
        let components: Vec<&str> = path.split('.').collect();
        
        // Navigate through the source object
        let mut current = source;
        for component in components {
            match current {
                Value::Object(map) => {
                    current = map.get(component).ok_or_else(|| {
                        UVError::PropertyMappingError(format!("Property not found: {}", component))
                    })?;
                },
                _ => {
                    return Err(UVError::PropertyMappingError(
                        format!("Cannot navigate through non-object: {}", current)
                    ));
                }
            }
        }
        
        // Return a clone of the value
        Ok(current.clone())
    }
    
    /// Insert a value into an object using a path expression.
    ///
    /// The path is a dot-separated list of property names, e.g. "user.address.city".
    fn insert_value(&self, target: &mut serde_json::Map<String, Value>, path: &str, value: Value) -> Result<()> {
        // Split the path into components
        let components: Vec<&str> = path.split('.').collect();
        
        // Navigate through the target object, creating objects as needed
        let mut current = target;
        for (i, component) in components.iter().enumerate() {
            if i == components.len() - 1 {
                // Last component, insert the value
                current.insert(component.to_string(), value);
                return Ok(());
            } else {
                // Not the last component, ensure the next level exists
                if !current.contains_key(*component) {
                    current.insert(component.to_string(), Value::Object(serde_json::Map::new()));
                }
                
                // Move to the next level
                match current.get_mut(*component) {
                    Some(Value::Object(map)) => {
                        current = map;
                    },
                    _ => {
                        return Err(UVError::PropertyMappingError(
                            format!("Cannot navigate through non-object: {}", component)
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
}
