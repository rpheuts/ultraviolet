//! Amazon Q prism implementation for the Ultraviolet system.
//!
//! This crate provides an Amazon Q AI model integration prism
//! that allows other prisms to access Amazon Q models via CLI.

pub mod spectrum;

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
use tokio::runtime::Runtime;
use uuid::Uuid;

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

        // Convert output to string
        let response = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(response)
    }

    /// Invoke Q CLI with streaming response using stdin
    async fn invoke_q_cli_stream(
        &self,
        model: &str,
        prompt: &str,
        _max_tokens: i32,
        id: Uuid,
        link: &UVLink,
    ) -> Result<()> {
        let model_str = model.to_string();
        let prompt_str = prompt.to_string();
        
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

        // Convert output to string and simulate streaming by emitting word by word
        let response = String::from_utf8_lossy(&output.stdout);
        
        // Split response into words and emit as tokens to simulate streaming
        let words: Vec<&str> = response.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            let token = if i == 0 { 
                word.to_string() 
            } else { 
                format!(" {}", word)
            };
            
            link.emit_photon(id, json!({"token": token}))?;
            
            // Add small delay to simulate streaming (optional)
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // Signal completion
        link.emit_trap(id, None)?;
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
