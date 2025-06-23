//! Spectrum types for the deploy prism.

use serde::{Deserialize, Serialize};

/// Lambda deployment request
#[derive(Debug, Deserialize)]
pub struct LambdaRequest {
    /// Name of the Lambda function
    pub function_name: String,
    /// AWS region
    #[serde(default = "default_region")]
    pub region: String,
    /// Lambda memory size in MB
    #[serde(default = "default_memory")]
    pub memory_size: i32,
    /// Lambda timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: i32,
    /// API Gateway stage name
    #[serde(default = "default_stage")]
    pub stage_name: String,
}

/// Lambda deployment response
#[derive(Debug, Serialize)]
pub struct LambdaResponse {
    /// ARN of the deployed Lambda function
    pub function_arn: String,
    /// ID of the API Gateway WebSocket API
    pub api_id: String,
    /// WebSocket URL for clients
    pub websocket_url: String,
    /// ISO timestamp of deployment
    pub deployment_time: String,
    /// Deployment status
    pub status: String,
}

/// Deployment status update
#[derive(Debug, Serialize)]
pub struct DeploymentStatus {
    /// Current status
    pub status: String,
    /// Status message
    pub message: String,
    /// Optional progress percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
}

// Default values
fn default_region() -> String {
    "us-west-2".to_string()
}

fn default_memory() -> i32 {
    256
}

fn default_timeout() -> i32 {
    30
}

fn default_stage() -> String {
    "prod".to_string()
}
