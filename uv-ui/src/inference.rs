//! Inference engine for detecting appropriate UI components from data

use std::collections::HashMap;
use serde_json::Value;

use crate::{
    UIComponent, PropertyValue, DataType, Card, Table, Column, List, Text, 
    TypeDetectors, UIError, Result
};

/// Engine for inferring UI components from data
pub struct UIInferenceEngine {
    /// Type detectors for identifying property types
    type_detectors: TypeDetectors,
}

impl UIInferenceEngine {
    /// Create a new inference engine with default detectors
    pub fn new() -> Self {
        Self {
            type_detectors: TypeDetectors::new(),
        }
    }
    
    /// Infer a UI component from a JSON value
    pub fn infer(&self, value: &Value) -> Result<UIComponent> {
        match value {
            Value::Object(_) => self.infer_object(value),
            Value::Array(_) => self.infer_array(value),
            Value::String(_) => self.infer_string(value),
            Value::Number(_) => self.infer_scalar(value),
            Value::Bool(_) => self.infer_scalar(value),
            Value::Null => Err(UIError::InferenceError("Cannot infer component from null".to_string())),
        }
    }
    
    /// Infer a component from an object
    fn infer_object(&self, value: &Value) -> Result<UIComponent> {
        let obj = value.as_object().unwrap(); // Safe because we checked in infer()
        let mut properties = HashMap::new();
        
        for (key, val) in obj {
            let prop_value = self.type_detectors.detect(val);
            properties.insert(key.clone(), prop_value);
        }
        
        // Create a card with the properties
        let mut card = Card::new().with_properties(properties.clone());
        
        // Try to infer a title from the properties
        if let Some(title) = infer_title_from_properties(&properties) {
            card = card.with_title(title);
        }
        
        Ok(UIComponent::Card(card))
    }
    
    /// Infer a component from an array
    fn infer_array(&self, value: &Value) -> Result<UIComponent> {
        let array = value.as_array().unwrap(); // Safe because we checked in infer()
        
        if array.is_empty() {
            // Return an empty table
            return Ok(UIComponent::Table(Table::new()));
        }
        
        // Check if all items are objects with similar structure (table)
        if array.iter().all(|item| item.is_object()) {
            return self.infer_table(array);
        }
        
        // Check if all items are strings (list)
        if array.iter().all(|item| item.is_string() || item.is_number() || item.is_boolean()) {
            return self.infer_list(array);
        }
        
        // Mixed array, default to list
        let items = array.iter()
            .map(|val| self.type_detectors.detect(val))
            .collect();
        
        Ok(UIComponent::List(List::new().with_items(items)))
    }
    
    /// Infer a table component from an array of objects
    fn infer_table(&self, array: &[Value]) -> Result<UIComponent> {
        // Extract column names from all objects
        let mut all_keys = HashMap::new();
        
        for item in array {
            let obj = item.as_object().unwrap(); // Safe because we checked in infer_array()
            for key in obj.keys() {
                all_keys.insert(key.clone(), true);
            }
        }
        
        // Sort keys for consistent ordering
        let mut column_names: Vec<String> = all_keys.keys().cloned().collect();
        column_names.sort();
        
        // Create columns
        let mut columns = Vec::new();
        for name in &column_names {
            // Try to detect the column type from the first non-null value in any row
            let mut data_type = DataType::Text; // Default type
            
            for item in array {
                if let Some(obj) = item.as_object() {
                    if let Some(val) = obj.get(name) {
                        if !val.is_null() {
                            // Use the type of this value
                            data_type = self.type_detectors.detect(val).data_type();
                            break;
                        }
                    }
                }
            }
            
            columns.push(Column {
                name: name.clone(),
                data_type,
            });
        }
        
        // Create rows
        let mut rows = Vec::new();
        for item in array {
            let obj = item.as_object().unwrap(); // Safe because we checked in infer_array()
            
            let mut row = Vec::new();
            for name in &column_names {
                if let Some(val) = obj.get(name) {
                    row.push(self.type_detectors.detect(val));
                } else {
                    // Missing value, add null
                    row.push(PropertyValue::Other(Value::Null));
                }
            }
            
            rows.push(row);
        }
        
        // Create the table
        let table = Table::new()
            .with_columns(columns)
            .with_rows(rows);
        
        Ok(UIComponent::Table(table))
    }
    
    /// Infer a list component from an array of scalars
    fn infer_list(&self, array: &[Value]) -> Result<UIComponent> {
        let items = array.iter()
            .map(|val| self.type_detectors.detect(val))
            .collect();
        
        Ok(UIComponent::List(List::new().with_items(items)))
    }
    
    /// Infer a component from a string value
    fn infer_string(&self, value: &Value) -> Result<UIComponent> {
        let text = value.as_str().unwrap().to_string(); // Safe because we checked in infer()
        let text_component = Text::new(text);
        
        // Try to detect format
        let text_component = if let Some(format) = text_component.detect_format() {
            text_component.with_format(format)
        } else {
            text_component
        };
        
        Ok(UIComponent::Text(text_component))
    }
    
    /// Infer a component from a scalar value
    fn infer_scalar(&self, value: &Value) -> Result<UIComponent> {
        // Convert the value to a string representation
        let text = match value {
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => return Err(UIError::InferenceError("Unexpected scalar type".to_string())),
        };
        
        Ok(UIComponent::Text(Text::new(text)))
    }
}

// Helper function to infer a title from an object's properties
fn infer_title_from_properties(properties: &HashMap<String, PropertyValue>) -> Option<String> {
    // Priority order for title fields
    for field in &["title", "name", "id", "key", "header", "subject"] {
        if let Some(PropertyValue::Text(text)) = properties.get(*field) {
            return Some(text.clone());
        }
    }
    
    None
}