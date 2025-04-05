//! Abstract UI component system for Ultraviolet
//!
//! This crate provides a set of abstract UI components that can be used to represent
//! data in a UI-agnostic way. The components are designed to be rendered by
//! concrete implementations in different environments (CLI, web, etc).
//!
//! The system uses inference to automatically convert JSON data into appropriate
//! UI components based on structure and content analysis.

use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde_json::Value;
use thiserror::Error;

/// Errors that can occur in UI operations
#[derive(Error, Debug)]
pub enum UIError {
    /// Error during value conversion
    #[error("Failed to convert value: {0}")]
    ConversionError(String),
    
    /// Error during component inference
    #[error("Failed to infer component: {0}")]
    InferenceError(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// Result type for UI operations
pub type Result<T> = std::result::Result<T, UIError>;

/// Abstract UI component types
#[derive(Debug, Clone)]
pub enum UIComponent {
    /// Card component for displaying a single object with properties
    Card(Card),
    
    /// Table component for displaying arrays of objects
    Table(Table),
    
    /// List component for displaying arrays of values
    List(List),
    
    /// Text component for displaying single text values
    Text(Text),
}

/// Data types for values
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Text,
    Number,
    Boolean,
    Date,
    Duration,
    URL,
    FilePath,
    Custom(String),
}

/// Property value with type information
#[derive(Debug, Clone)]
pub enum PropertyValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    Date(DateTime<Utc>),
    Duration(std::time::Duration),
    URL(String),
    FilePath(PathBuf),
    /// For values that don't fit into other categories
    Other(Value),
    /// Nested UI component for hierarchical structure
    Component(Box<UIComponent>),
}

impl PropertyValue {
    /// Get the data type of the property value
    pub fn data_type(&self) -> DataType {
        match self {
            PropertyValue::Text(_) => DataType::Text,
            PropertyValue::Number(_) => DataType::Number,
            PropertyValue::Boolean(_) => DataType::Boolean,
            PropertyValue::Date(_) => DataType::Date,
            PropertyValue::Duration(_) => DataType::Duration,
            PropertyValue::URL(_) => DataType::URL,
            PropertyValue::FilePath(_) => DataType::FilePath,
            PropertyValue::Other(_) => DataType::Custom("Other".to_string()),
            PropertyValue::Component(_) => DataType::Custom("Component".to_string()),
        }
    }
}

/// Card component for displaying a single object
#[derive(Debug, Clone)]
pub struct Card {
    /// Optional title for the card
    pub title: Option<String>,
    
    /// Map of property names to values
    pub properties: HashMap<String, PropertyValue>,
    
    /// Additional metadata for the card
    pub metadata: HashMap<String, Value>,
}

/// Table component for displaying arrays of objects
#[derive(Debug, Clone)]
pub struct Table {
    /// Column definitions
    pub columns: Vec<Column>,
    
    /// Rows of data
    pub rows: Vec<Vec<PropertyValue>>,
    
    /// Additional metadata for the table
    pub metadata: HashMap<String, Value>,
}

/// Column definition for tables
#[derive(Debug, Clone)]
pub struct Column {
    /// Name of the column
    pub name: String,
    
    /// Data type for the column
    pub data_type: DataType,
}

/// List component for displaying arrays of values
#[derive(Debug, Clone)]
pub struct List {
    /// Items in the list
    pub items: Vec<PropertyValue>,
    
    /// Additional metadata for the list
    pub metadata: HashMap<String, Value>,
}

/// Text component for displaying single text values
#[derive(Debug, Clone)]
pub struct Text {
    /// The text content
    pub content: String,
    
    /// Optional format hint (plain, markdown, json, etc.)
    pub format: Option<String>,
    
    /// Additional metadata for the text
    pub metadata: HashMap<String, Value>,
}

/// Interface for type detection
pub trait TypeDetector {
    /// Attempt to detect and convert a value to a PropertyValue
    fn detect(&self, value: &Value) -> Option<PropertyValue>;
}

mod detectors;
mod components;
mod inference;

pub use detectors::TypeDetectors;
pub use components::*;
pub use inference::UIInferenceEngine;

// Re-exports for convenience
pub mod prelude {
    pub use crate::{
        UIComponent, PropertyValue, DataType,
        Card, Table, Column, List, Text,
        TypeDetector, UIInferenceEngine,
        Result, UIError,
    };
}
