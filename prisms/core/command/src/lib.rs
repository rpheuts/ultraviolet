//! Command execution prism implementation for the Ultraviolet system.
//!
//! This crate provides a command execution prism that allows running
//! local shell commands with both synchronous and streaming output modes.

pub mod spectrum;

use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use spectrum::{ExecRequest, ExecResponse, ShellExecRequest};
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// Command execution prism for running local shell commands.
pub struct CommandPrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
    /// Tokio runtime for async operations
    runtime: Runtime,
}

impl CommandPrism {
    /// Create a new Command prism.
    pub fn new() -> Result<Self> {
        // Create a tokio runtime
        let runtime = Runtime::new()
            .map_err(|e| UVError::ExecutionError(format!("Failed to create runtime: {}", e)))?;

        Ok(Self {
            spectrum: None,
            runtime,
        })
    }

    /// Execute a command and return complete output
    async fn exec_command(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        timeout_seconds: u64,
    ) -> Result<(String, String, i32)> {
        // Build the command
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args);
        
        // Set working directory if provided
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        
        // Set environment variables if provided
        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }
        
        // Configure stdio
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        
        // Create timeout duration
        let timeout = Duration::from_secs(timeout_seconds);
        
        // Execute with timeout
        let result = tokio::time::timeout(
            timeout,
            async {
                let output = cmd.output().await?;
                
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                
                Ok::<(String, String, i32), std::io::Error>((stdout, stderr, exit_code))
            }
        ).await;
        
        match result {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(e)) => Err(UVError::ExecutionError(format!("Command execution failed: {}", e))),
            Err(_) => Err(UVError::ExecutionError(format!("Command timed out after {} seconds", timeout_seconds))),
        }
    }

    /// Execute a command with streaming output
    async fn exec_command_stream(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        timeout_seconds: u64,
        id: Uuid,
        link: &UVLink,
    ) -> Result<()> {
        // Build the command
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args);
        
        // Set working directory if provided
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        
        // Set environment variables if provided
        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }
        
        // Configure stdio
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        
        // Spawn the process
        let mut child = cmd.spawn()
            .map_err(|e| UVError::ExecutionError(format!("Failed to spawn command: {}", e)))?;
        
        // Get stdout and stderr pipes
        let stdout = child.stdout.take()
            .ok_or_else(|| UVError::ExecutionError("Failed to capture stdout".into()))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| UVError::ExecutionError("Failed to capture stderr".into()))?;
        
        // Create readers for stdout and stderr
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);
        
        let mut stdout_lines = stdout_reader.lines();
        let mut stderr_lines = stderr_reader.lines();
        
        // Create timeout duration
        let timeout = Duration::from_secs(timeout_seconds);
        
        // Process output with timeout
        let stream_result = tokio::time::timeout(
            timeout,
            async {
                loop {
                    tokio::select! {
                        stdout_result = stdout_lines.next_line() => {
                            match stdout_result {
                                Ok(Some(line)) => {
                                    link.emit_photon(id, json!({
                                        "line": line,
                                        "source": "stdout"
                                    }))?;
                                },
                                Ok(None) => {
                                    // stdout closed
                                    break;
                                },
                                Err(e) => {
                                    return Err(UVError::ExecutionError(format!("Error reading stdout: {}", e)));
                                }
                            }
                        },
                        stderr_result = stderr_lines.next_line() => {
                            match stderr_result {
                                Ok(Some(line)) => {
                                    link.emit_photon(id, json!({
                                        "line": line,
                                        "source": "stderr"
                                    }))?;
                                },
                                Ok(None) => {
                                    // stderr closed
                                    break;
                                },
                                Err(e) => {
                                    return Err(UVError::ExecutionError(format!("Error reading stderr: {}", e)));
                                }
                            }
                        },
                    }
                }
                Ok::<(), UVError>(())
            }
        ).await;
        
        // Check if we timed out during streaming
        match stream_result {
            Ok(Ok(())) => {
                // Streaming completed successfully, now wait for process to finish
                let status = child.wait().await
                    .map_err(|e| UVError::ExecutionError(format!("Error waiting for process: {}", e)))?;
                
                let exit_code = status.code().unwrap_or(-1);
                
                // Emit final system message with exit code
                link.emit_photon(id, json!({
                    "line": format!("Process exited with code {}", exit_code),
                    "source": "system"
                }))?;
                
                // Signal completion
                link.emit_trap(id, None)?;
                Ok(())
            },
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Kill the process if it's still running
                let _ = child.kill().await;
                Err(UVError::ExecutionError(format!("Command timed out after {} seconds", timeout_seconds)))
            }
        }
    }

    /// Handle 'exec' frequency
    fn handle_exec(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: ExecRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        // Execute the command using the tokio runtime
        let result = self.runtime.block_on(async {
            self.exec_command(
                &request.command,
                &request.args,
                request.cwd.as_deref(),
                request.env.as_ref(),
                request.timeout_seconds,
            ).await
        });

        match result {
            Ok((stdout, stderr, exit_code)) => {
                let response = ExecResponse {
                    stdout,
                    stderr,
                    exit_code,
                    success: exit_code == 0,
                };

                // Emit the response
                link.emit_photon(id, serde_json::to_value(response)?)?;
                
                // Signal successful completion
                link.emit_trap(id, None)?;
            },
            Err(e) => {
                // Signal error
                link.emit_trap(id, Some(e))?;
            }
        }

        Ok(())
    }

    /// Handle 'exec_stream' frequency
    fn handle_exec_stream(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: ExecRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        // Execute the command with streaming using the tokio runtime
        let result = self.runtime.block_on(async {
            self.exec_command_stream(
                &request.command,
                &request.args,
                request.cwd.as_deref(),
                request.env.as_ref(),
                request.timeout_seconds,
                id,
                link,
            ).await
        });

        if let Err(e) = result {
            // Signal error (if we haven't already sent a trap)
            let _ = link.emit_trap(id, Some(e));
        }

        Ok(())
    }

    /// Handle 'shell_exec_stream' frequency
    fn handle_shell_exec_stream(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: ShellExecRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        // Execute the shell command using sh -c with streaming
        let shell_command = "sh";
        let shell_args = vec!["-c".to_string(), request.shell_command];

        let result = self.runtime.block_on(async {
            self.exec_command_stream(
                shell_command,
                &shell_args,
                request.cwd.as_deref(),
                request.env.as_ref(),
                request.timeout_seconds,
                id,
                link,
            ).await
        });

        if let Err(e) = result {
            // Signal error (if we haven't already sent a trap)
            let _ = link.emit_trap(id, Some(e));
        }

        Ok(())
    }
}

impl UVPrism for CommandPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "exec" => {
                    self.handle_exec(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "exec_stream" => {
                    self.handle_exec_stream(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "shell_exec_stream" => {
                    self.handle_shell_exec_stream(id, wavefront.input.clone(), link)?;
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
    Box::new(CommandPrism::new().unwrap())
}
