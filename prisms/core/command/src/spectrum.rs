use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request input for command execution methods
#[derive(Debug, Deserialize)]
pub struct ExecRequest {
    /// The command to execute
    pub command: String,
    /// Optional command arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Optional working directory
    pub cwd: Option<String>,
    /// Optional environment variables
    pub env: Option<HashMap<String, String>>,
    /// Command timeout in seconds (default: 60)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

/// Request input for shell command execution methods
#[derive(Debug, Deserialize)]
pub struct ShellExecRequest {
    /// The complete shell command string to execute
    pub shell_command: String,
    /// Optional working directory
    pub cwd: Option<String>,
    /// Optional environment variables
    pub env: Option<HashMap<String, String>>,
    /// Command timeout in seconds (default: 60)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

/// Response from command execution
#[derive(Debug, Serialize)]
pub struct ExecResponse {
    /// Standard output from the command
    pub stdout: String,
    /// Standard error from the command
    pub stderr: String,
    /// Exit code of the command
    pub exit_code: i32,
    /// Whether the command executed successfully (exit code 0)
    pub success: bool,
}

/// Streaming output line from command execution
#[derive(Debug, Serialize)]
pub struct StreamLine {
    /// Content of the output line
    pub line: String,
    /// Source of the line (stdout, stderr, or system)
    pub source: String,
}

/// Default timeout value: 60 seconds
fn default_timeout() -> u64 {
    60
}

/// Default timeout duration
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 60;
