//! Typed structures for the oppie prism spectrum.
//!
//! This module defines the input and output structures for the oppie prism,
//! providing type safety and better developer experience.

use serde::{Deserialize, Serialize};

/// Input for the extract frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractInput {
    /// Comma-separated list of usernames
    pub users: String,
    
    /// Optional fleet ID for FUA service
    #[serde(default)]
    pub fleet_id: Option<String>,
    
    /// Comma-separated list of services
    #[serde(default = "default_services")]
    pub services: String,
}

/// Input for the upload frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadInput {
    /// Comma-separated list of usernames
    pub users: String,
    
    /// Optional fleet ID for FUA service
    #[serde(default)]
    pub fleet_id: Option<String>,
    
    /// Comma-separated list of services
    #[serde(default = "default_upload_services")]
    pub services: String,
}

/// Progress message for streaming output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    /// Message text
    pub message: String,
    
    /// Service name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    
    /// User being processed (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    
    /// Progress percentage (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
}

/// Common HTTP response structure (from curl prism)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: i32,
    
    /// Response body
    pub body: String,
}

/// Default services for extract
fn default_services() -> String {
    "sas,shepherd,policy_engine,cti,org,asr".to_string()
}

/// Default services for upload
fn default_upload_services() -> String {
    "sas,shepherd,policy_engine,cti,org,fua,asr".to_string()
}
