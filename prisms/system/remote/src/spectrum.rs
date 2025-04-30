use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request for the refract frequency
#[derive(Debug, Deserialize)]
pub struct RefractRequest {
    /// The WebSocket URL of the remote Ultraviolet server
    pub url: String,
    
    /// The target prism identifier
    pub prism: String,
    
    /// The target frequency to invoke
    pub frequency: String,
    
    /// The input data to send to the target prism
    pub input: Option<Value>,
}

/// Response from a remote prism
#[derive(Debug, Serialize)]
pub struct RefractResponse {
    /// The response data from the remote prism
    pub data: Value,
}
