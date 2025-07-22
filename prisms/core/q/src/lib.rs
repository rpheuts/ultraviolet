//! Amazon Q prism implementation for the Ultraviolet system.
//!
//! This crate provides an Amazon Q AI model integration prism
//! that allows other prisms to access Amazon Q models via CLI.

pub mod spectrum;

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use tokio::runtime::Runtime;
use uuid::Uuid;
use regex::Regex;

use spectrum::{DEFAULT_MODEL, InvokeRequest};
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// Q prism for accessing Amazon Q AI models via CLI.
pub struct QPrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
    /// Tokio runtime for async operations
    runtime: Runtime,
}

impl QPrism {
    /// Create a new Q prism.
    pub fn new() -> Result<Self> {
        // Create a tokio runtime
        let runtime = Runtime::new()
            .map_err(|e| UVError::ExecutionError(format!("Failed to create runtime: {}", e)))?;
        
        Ok(Self {
            spectrum: None,
            runtime,
        })  
    }

    /// Invoke Q CLI with the given prompt (non-streaming) using stdin
    async fn invoke_q_cli(
        &self,
        model: &str,
        prompt: &str,
        _max_tokens: i32, // Q CLI may not support max_tokens directly
    ) -> Result<String> {
        let model_str = model.to_string();
        let prompt_str = prompt.to_string();

        // Static ANSI and control character cleaning function
        let clean_chunk = |chunk: &str| -> String {
            let ansi_regex = Regex::new(r"\x1b\[[0-9;]*[mK]").unwrap();
            let ansi_cleaned = ansi_regex.replace_all(chunk, "");
            // Remove all control characters (0x00-0x1F) except newlines and tabs
            ansi_cleaned.chars()
                .filter(|&c| c >= ' ' || c == '\n' || c == '\t')
                .collect()
        };
        
        // Execute Q CLI command with stdin piping
        let output = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new("q");
            cmd.args(&["chat", "--no-interactive"]);
            
            // Add model parameter if not default
            if model_str != DEFAULT_MODEL {
                cmd.args(&["--model", &model_str]);
            }
            
            // Configure stdin/stdout
            cmd.stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .stderr(Stdio::piped());
            
            // Spawn the process
            let mut child = cmd.spawn()
                .map_err(|e| UVError::ExecutionError(format!("Failed to start Q CLI: {}", e)))?;
            
            // Write prompt to stdin
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(prompt_str.as_bytes())
                    .map_err(|e| UVError::ExecutionError(format!("Failed to write to Q CLI stdin: {}", e)))?;
                stdin.flush()
                    .map_err(|e| UVError::ExecutionError(format!("Failed to flush Q CLI stdin: {}", e)))?;
                // Close stdin to signal EOF
                drop(stdin);
            }
            
            // Wait for output
            child.wait_with_output()
                .map_err(|e| UVError::ExecutionError(format!("Failed to read Q CLI output: {}", e)))
        }).await
        .map_err(|e| UVError::ExecutionError(format!("Failed to spawn Q CLI task: {}", e)))??;

        // Check if the command was successful
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(UVError::ExecutionError(format!(
                "Q CLI command failed: {} (Make sure 'q' CLI tool is installed and available in PATH)",
                stderr
            )));
        }

        // Convert output to string and clean it
        let raw_response = String::from_utf8_lossy(&output.stdout).to_string();
        let clean_response = clean_chunk(&raw_response);
        
        Ok(clean_response)
    }

    /// Invoke Q CLI with real streaming response
    async fn invoke_q_cli_stream(
        &self,
        model: &str,
        prompt: &str,
        _max_tokens: i32,
        id: Uuid,
        link: &UVLink,
    ) -> Result<()> {
        use std::io::{BufReader};
        use std::process::Stdio;
        
        let model_str = model.to_string();
        let prompt_str = prompt.to_string();
        let link_clone = link.clone();
        
        tokio::task::spawn_blocking(move || {
            // Static ANSI and control character cleaning function
            let clean_chunk = |chunk: &str| -> String {
                let ansi_regex = Regex::new(r"\x1b\[[0-9;]*[mK]").unwrap();
                let ansi_cleaned = ansi_regex.replace_all(chunk, "");
                // Remove all control characters (0x00-0x1F) except newlines and tabs
                let control_cleaned: String = ansi_cleaned.chars()
                    .filter(|&c| c >= ' ' || c == '\n' || c == '\t' || c == '\r')
                    .collect();
                let mut cleaned = control_cleaned.replace("\r\n", "\n").replace("\r", "\n").replace("•", "\n •");

                if cleaned.starts_with(">") {
                    cleaned = cleaned.replacen(">", "", 1);
                }

                return cleaned;
            };
            
            let mut cmd = Command::new("q");
            cmd.args(&["chat", "--no-interactive"]);
            
            // Add model parameter if not default
            if model_str != DEFAULT_MODEL {
                cmd.args(&["--model", &model_str]);
            }
            
            // Configure stdin/stdout
            cmd.stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .stderr(Stdio::piped());
            
            // Spawn the process
            let mut child = cmd.spawn()
                .map_err(|e| UVError::ExecutionError(format!("Failed to start Q CLI: {}", e)))?;
            
            // Write prompt to stdin
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(prompt_str.as_bytes())
                    .map_err(|e| UVError::ExecutionError(format!("Failed to write to Q CLI stdin: {}", e)))?;
                stdin.flush()
                    .map_err(|e| UVError::ExecutionError(format!("Failed to flush Q CLI stdin: {}", e)))?;
                drop(stdin);
            }
            
            // Stream stdout in real-time while also capturing stderr
            let mut stderr_buffer = Vec::new();
            let mut tokens_sent = false;
            
            if let Some(stdout) = child.stdout.take() {
                if let Some(mut stderr) = child.stderr.take() {
                    // Read stderr in background to prevent blocking
                    let stderr_handle = std::thread::spawn(move || {
                        let mut buffer = Vec::new();
                        stderr.read_to_end(&mut buffer).unwrap_or(0);
                        buffer
                    });
                    
                    // Stream stdout
                    let reader = BufReader::new(stdout);
                    let mut buffer = [0; 256]; // Small buffer for streaming
                    let mut stdout_reader = reader.into_inner();
                    
                    loop {
                        match stdout_reader.read(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                let chunk = String::from_utf8_lossy(&buffer[..n]);
                                let cleaned_chunk = clean_chunk(&chunk);
                                
                                if !cleaned_chunk.is_empty() {
                                    link_clone.emit_photon(id, json!({"token": cleaned_chunk}))
                                        .map_err(|e| UVError::ExecutionError(format!("Failed to emit photon: {}", e)))?;
                                    tokens_sent = true; // Track that we sent at least one token
                                }
                            }
                            Err(e) => {
                                return Err(UVError::ExecutionError(format!("Failed to read stdout: {}", e)));
                            }
                        }
                    }
                    
                    // Get stderr content
                    stderr_buffer = stderr_handle.join().unwrap_or_default();
                }
            }
            
            // Wait for process to complete
            let status = child.wait()
                .map_err(|e| UVError::ExecutionError(format!("Failed to wait for Q CLI: {}", e)))?;
            
            // Smart error detection: if no tokens were sent, check stderr for error messages
            if !tokens_sent && !stderr_buffer.is_empty() {
                let stderr = String::from_utf8_lossy(&stderr_buffer);
                
                // Filter out normal Q CLI chatter and extract actual error messages
                let error_lines: Vec<&str> = stderr
                    .lines()
                    .filter(|line| {
                        let line = line.trim();
                        // Skip normal Q CLI status messages
                        !line.is_empty()
                    })
                    .collect();
                
                if !error_lines.is_empty() {
                    let error_msg = format!("Q CLI Error: {}", error_lines.join(" "));
                    link_clone.emit_trap(id, Some(UVError::ExecutionError(error_msg)))?;
                    return Ok(());
                }
            }
            
            // Also check traditional exit status (in case Q CLI fixes their behavior)
            if !status.success() {
                let stderr = String::from_utf8_lossy(&stderr_buffer);
                let error_msg = if stderr.trim().is_empty() {
                    "Q CLI command failed".to_string()
                } else {
                    format!("Q CLI Error: {}", stderr.trim())
                };
                link_clone.emit_trap(id, Some(UVError::ExecutionError(error_msg)))?;
                return Ok(());
            }

            link_clone.emit_trap(id, None)?;
            
            Ok(())
        }).await
        .map_err(|e| UVError::ExecutionError(format!("Failed to spawn streaming task: {}", e)))??;

        Ok(())
    }

    /// Handle 'invoke' frequency
    fn handle_invoke(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: InvokeRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        // Get the model ID (for future compatibility)
        let model = request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());

        // Invoke Q CLI using the tokio runtime
        let response = self.runtime.block_on(async {
            self.invoke_q_cli(&model, &request.prompt, request.max_tokens).await
        })?;

        // Emit the response
        link.emit_photon(id, json!({ "response": response }))?;

        // Signal successful completion
        link.emit_trap(id, None)?;

        Ok(())
    }

    /// Handle 'invoke_stream' frequency
    fn handle_invoke_stream(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: InvokeRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        // Get the model ID (for future compatibility)
        let model = request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());

        // Invoke Q CLI with streaming using the tokio runtime
        self.runtime.block_on(async {
            self.invoke_q_cli_stream(&model, &request.prompt, request.max_tokens, id, link).await
        })?;

        Ok(())
    }
}

impl UVPrism for QPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "invoke" => {
                    self.handle_invoke(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "invoke_stream" => {
                    self.handle_invoke_stream(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                _ => {
                    // Unknown frequency
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    return Ok(true);
                }
            }
        }
        
        // Ignore other pulse types
        Ok(false)
    }
}

#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(QPrism::new().unwrap())
}
