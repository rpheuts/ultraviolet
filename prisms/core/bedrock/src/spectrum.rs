use serde::{Deserialize, Serialize};

/// Request input for the invoke and invoke_stream methods
#[derive(Debug, Deserialize)]
pub struct InvokeRequest {
    /// The prompt text to send to the model
    pub prompt: String,
    /// Optional model ID (defaults to Claude 3.5 Sonnet)
    pub model: Option<String>,
    /// Maximum number of tokens to generate (default: 4096)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
}

/// Request input for the invoke_with_files method
#[derive(Debug, Deserialize)]
pub struct InvokeWithFilesRequest {
    /// The prompt text to send to the model
    pub prompt: String,
    /// Optional model ID (defaults to Claude 3.5 Sonnet)
    pub model: Option<String>,
    /// Maximum number of tokens to generate (default: 4096)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
    /// File paths to include as context
    pub files: Vec<String>,
}

/// Response from a Bedrock invocation
#[derive(Debug, Serialize, Deserialize)]
pub struct InvokeResponse {
    /// The text response from the model
    pub response: String,
}

/// Default maximum tokens value: 4096
fn default_max_tokens() -> i32 {
    4096
}

/// Claude 3.5 Sonnet Bedrock model ID
pub const DEFAULT_MODEL: &str = "anthropic.claude-3-5-sonnet-20241022-v2:0";

/// Default AWS region for Bedrock
pub const DEFAULT_REGION: &str = "us-west-2";
