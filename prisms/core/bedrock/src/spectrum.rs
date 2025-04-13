use serde::{Deserialize, Serialize};

/// Request input for the invoke and invoke_stream methods
#[derive(Debug, Deserialize)]
pub struct InvokeRequest {
    /// The prompt text to send to the model
    pub prompt: String,
    /// Optional model ID (defaults to Claude 3.7 Sonnet)
    pub model: Option<String>,
    /// Maximum number of tokens to generate (default: 4096)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
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

/// Claude 3.7 Sonnet Bedrock model ID
pub const DEFAULT_MODEL: &str = "us.anthropic.claude-3-7-sonnet-20250219-v1:0";

/// Default AWS region for Bedrock
pub const DEFAULT_REGION: &str = "us-west-2";

/// Available models for the model selector
pub const AVAILABLE_MODELS: &[(&str, &str)] = &[
    ("Claude 3.7", "us.anthropic.claude-3-7-sonnet-20250219-v1:0"),
    ("Claude 3.5", "us.anthropic.claude-3-5-sonnet-20241022-v2:0"),
    ("DeepSeek R1", "us.deepseek.r1-v1:0"),
    ("Llama 3", "us.meta.llama3-1-405b-instruct-v1:0"),
    ("AWS Nova", "us.amazon.nova-pro-v1:0"),
];
