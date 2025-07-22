use serde::{Deserialize, Serialize};
use std::env;

/// Request input for the invoke and invoke_stream methods
#[derive(Debug, Deserialize)]
pub struct InvokeRequest {
    /// The prompt text to send to the model
    pub prompt: String,
    /// Optional model ID (defaults to llama2)
    pub model: Option<String>,
    /// Maximum number of tokens to generate (default: 4096)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
}

/// Response from an Ollama invocation
#[derive(Debug, Serialize, Deserialize)]
pub struct InvokeResponse {
    /// The text response from the model
    pub response: String,
}

/// Default maximum tokens value: 4096
fn default_max_tokens() -> i32 {
    4096
}

/// Default model: llama2 (popular and widely available)
pub const DEFAULT_MODEL: &str = "llama2";

/// Default Ollama base URL
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";

/// Get the Ollama API endpoint URL
pub fn get_ollama_api_url() -> String {
    let base_url = env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_OLLAMA_BASE_URL.to_string());
    
    // Remove trailing slash if present
    let base_url = base_url.trim_end_matches('/');
    
    format!("{}/api/chat", base_url)
}
