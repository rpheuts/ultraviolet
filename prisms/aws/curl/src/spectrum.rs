//! Typed structures for the curl prism spectrum.
//!
//! This module defines the input and output structures for the curl prism,
//! providing type safety and better developer experience.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input for the GET frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInput {
    /// URL to make the GET request to
    pub url: String,
    
    /// Optional HTTP headers as key-value pairs
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

/// Input for the POST frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInput {
    /// URL to make the POST request to
    pub url: String,
    
    /// Request body to send
    pub body: String,
    
    /// HTTP method to use (default: POST)
    #[serde(default)]
    pub method: Option<String>,
    
    /// Optional HTTP headers as key-value pairs
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

/// Common HTTP response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: i32,
    
    /// Response body
    pub body: String,
}
