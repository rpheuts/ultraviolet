use serde::Deserialize;

/// Request input for the wait frequency
#[derive(Debug, Deserialize)]
pub struct WaitRequest {
    /// Duration to wait in milliseconds
    pub duration_ms: u64,
    /// Optional message to emit while waiting
    pub message: Option<String>,
}