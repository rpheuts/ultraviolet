use serde::Deserialize;

/// Default model ID for Q (if applicable)
pub const DEFAULT_MODEL: &str = "default";

/// Request format for invoking Q
#[derive(Debug, Deserialize)]
pub struct InvokeRequest {
    /// Optional model ID override
    pub model: Option<String>,
    /// Input prompt for the AI model
    pub prompt: String,
    /// Maximum tokens to generate (may not be supported by Q CLI)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
}

/// Default max_tokens value
fn default_max_tokens() -> i32 {
    4096
}
