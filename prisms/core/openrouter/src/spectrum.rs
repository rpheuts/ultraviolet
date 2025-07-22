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

/// Response from an OpenRouter invocation
#[derive(Debug, Serialize, Deserialize)]
pub struct InvokeResponse {
    /// The text response from the model
    pub response: String,
}

/// Default maximum tokens value: 4096
fn default_max_tokens() -> i32 {
    4096
}

/// Default model: Claude 3.5 Sonnet via OpenRouter
pub const DEFAULT_MODEL: &str = "anthropic/claude-3.5-sonnet";

/// OpenRouter API endpoint
pub const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// OpenRouter streaming API endpoint
pub const OPENROUTER_STREAM_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
