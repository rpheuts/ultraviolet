//! Components module for working with UI components

use std::collections::HashMap;
use serde_json::Value;

use crate::{Card, Table, Column, List, Text, PropertyValue, DataType, UIError, Result};

impl Card {
    /// Create a new Card component
    pub fn new() -> Self {
        Self {
            title: None,
            properties: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Set the title of the card
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    
    /// Add a property to the card
    pub fn with_property(mut self, name: impl Into<String>, value: PropertyValue) -> Self {
        self.properties.insert(name.into(), value);
        self
    }
    
    /// Add multiple properties to the card
    pub fn with_properties(mut self, properties: HashMap<String, PropertyValue>) -> Self {
        self.properties.extend(properties);
        self
    }
    
    /// Add metadata to the card
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
    
    /// Get a property from the card
    pub fn get_property(&self, name: &str) -> Option<&PropertyValue> {
        self.properties.get(name)
    }
    
    /// Find a suitable title from the card's properties
    pub fn infer_title(&self) -> Option<&str> {
        // If title is already set, return it
        if let Some(ref title) = self.title {
            return Some(title);
        }
        
        // Try common field names that could be used as a title
        for field in &["name", "title", "id", "key", "label"] {
            if let Some(PropertyValue::Text(text)) = self.properties.get(*field) {
                return Some(text);
            }
        }
        
        // No suitable title found
        None
    }
}

impl Table {
    /// Create a new Table component
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Add a column to the table
    pub fn with_column(mut self, name: impl Into<String>, data_type: DataType) -> Self {
        self.columns.push(Column {
            name: name.into(),
            data_type,
        });
        self
    }
    
    /// Add multiple columns to the table
    pub fn with_columns(mut self, columns: Vec<Column>) -> Self {
        self.columns.extend(columns);
        self
    }
    
    /// Add a row of values to the table
    pub fn with_row(mut self, row: Vec<PropertyValue>) -> Self {
        // Validate that the row length matches the number of columns
        if !self.columns.is_empty() && row.len() != self.columns.len() {
            // If lengths don't match, pad with nulls or truncate
            let mut adjusted_row = Vec::with_capacity(self.columns.len());
            for i in 0..self.columns.len() {
                if i < row.len() {
                    adjusted_row.push(row[i].clone());
                } else {
                    adjusted_row.push(PropertyValue::Other(Value::Null));
                }
            }
            self.rows.push(adjusted_row);
        } else {
            self.rows.push(row);
        }
        self
    }
    
    /// Add multiple rows to the table
    pub fn with_rows(mut self, rows: Vec<Vec<PropertyValue>>) -> Self {
        for row in rows {
            self = self.with_row(row);
        }
        self
    }
    
    /// Add metadata to the table
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
    
    /// Get a cell value from the table
    pub fn get_cell(&self, row: usize, column: usize) -> Option<&PropertyValue> {
        self.rows.get(row).and_then(|r| r.get(column))
    }
    
    /// Find the index of a column by name
    pub fn find_column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }
}

impl List {
    /// Create a new List component
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Add an item to the list
    pub fn with_item(mut self, item: PropertyValue) -> Self {
        self.items.push(item);
        self
    }
    
    /// Add multiple items to the list
    pub fn with_items(mut self, items: Vec<PropertyValue>) -> Self {
        self.items.extend(items);
        self
    }
    
    /// Add metadata to the list
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl Text {
    /// Create a new Text component
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            format: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Set the format of the text
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
    
    /// Add metadata to the text
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
    
    /// Detect the format of the text content
    pub fn detect_format(&self) -> Option<String> {
        // If format is already set, return it
        if let Some(ref format) = self.format {
            return Some(format.clone());
        }
        
        let content = self.content.trim();
        
        // Check for JSON
        if (content.starts_with('{') && content.ends_with('}')) || 
           (content.starts_with('[') && content.ends_with(']')) {
            if serde_json::from_str::<Value>(content).is_ok() {
                return Some("json".to_string());
            }
        }
        
        // Check for Markdown (basic check)
        if content.contains('#') || content.contains('*') || 
           content.contains('-') || content.contains('`') || 
           content.contains('[') && content.contains(']') && content.contains('(') && content.contains(')') {
            return Some("markdown".to_string());
        }
        
        // Default to plain text
        Some("text".to_string())
    }
}

/// Convert a PropertyValue to a Value
pub fn property_to_value(prop: &PropertyValue) -> Result<Value> {
    match prop {
        PropertyValue::Text(s) => Ok(Value::String(s.clone())),
        PropertyValue::Number(n) => Ok(Value::Number(serde_json::Number::from_f64(*n)
            .ok_or_else(|| UIError::ConversionError("Failed to convert number".to_string()))?)),
        PropertyValue::Boolean(b) => Ok(Value::Bool(*b)),
        PropertyValue::Date(dt) => Ok(Value::String(dt.to_rfc3339())),
        PropertyValue::Duration(d) => Ok(Value::Number(serde_json::Number::from_f64(d.as_secs_f64())
            .ok_or_else(|| UIError::ConversionError("Failed to convert duration".to_string()))?)),
        PropertyValue::URL(s) => Ok(Value::String(s.clone())),
        PropertyValue::FilePath(p) => Ok(Value::String(p.to_string_lossy().to_string())),
        PropertyValue::Other(v) => Ok(v.clone()),
        PropertyValue::Component(_) => Ok(Value::String("[nested component]".to_string())),
    }
}
