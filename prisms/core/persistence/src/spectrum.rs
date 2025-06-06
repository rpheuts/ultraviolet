//! Spectrum definitions for the persistence prism.

use serde::{Deserialize, Serialize};

/// Request to store photons to a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreRequest {
    /// Relative file path for storage
    pub path: String,
    /// Content type for storage format
    #[serde(default = "default_content_type")]
    pub content_type: String,
}

/// Response from a store operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Number of photons that were stored
    pub photons_stored: usize,
    /// Number of bytes written to file
    pub bytes_written: usize,
    /// The path where data was stored
    pub path: String,
    /// Content type used for storage
    pub content_type: String,
    /// Storage mode that was applied
    pub storage_mode: String,
}

/// Request to load photons from a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadRequest {
    /// Relative file path to load from
    pub path: String,
    /// Content type for parsing (optional, auto-detected if not specified)
    pub content_type: Option<String>,
}

/// Storage mode enumeration
#[derive(Debug, Clone)]
pub enum StorageMode {
    /// Store as plain text by extracting specified field
    Text { field: String },
    /// Store complete photon objects (default)
    UvPhoton,
}

impl StorageMode {
    /// Parse content type string into storage mode
    pub fn from_content_type(content_type: &str) -> Self {
        match content_type.split_once('/') {
            Some(("text", field)) => StorageMode::Text { field: field.to_string() },
            Some(("uv", "photon")) => StorageMode::UvPhoton,
            _ => StorageMode::UvPhoton, // Default fallback
        }
    }
    
    /// Get the string representation of the storage mode
    pub fn to_string(&self) -> String {
        match self {
            StorageMode::Text { field } => format!("text (field: {})", field),
            StorageMode::UvPhoton => "uv/photon".to_string(),
        }
    }
}

/// Default content type
fn default_content_type() -> String {
    "uv/photon".to_string()
}
