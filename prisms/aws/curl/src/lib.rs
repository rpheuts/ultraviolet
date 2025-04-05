//! Curl prism implementation for the Ultraviolet system.
//!
//! This crate provides a curl-based HTTP client prism that makes requests
//! with Amazon internal authentication.

pub mod spectrum;

use std::env;
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

use crate::spectrum::{GetInput, HttpResponse, PostInput};

/// Curl prism for making HTTP requests with Amazon internal auth.
pub struct CurlPrism {
    spectrum: Option<UVSpectrum>,
    home_dir: PathBuf,
}

impl CurlPrism {
    /// Create a new curl prism.
    pub fn new() -> Self {
        let home_dir = env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        
        Self {
            spectrum: None,
            home_dir,
        }
    }
    
    /// Get common header arguments for curl commands.
    fn get_headers_args(&self, headers: &Option<std::collections::HashMap<String, String>>) -> Vec<String> {
        let mut header_args = Vec::new();
        
        // Add default Content-Type for POST requests
        header_args.push("-H".to_string());
        header_args.push("Content-Type: application/json".to_string());
        
        // Add custom headers if present
        if let Some(headers) = headers {
            for (key, value) in headers {
                header_args.push("-H".to_string());
                header_args.push(format!("{}: {}", key, value));
            }
        }
        
        header_args
    }
    
    /// Execute a curl command and return the status code and response body.
    fn execute_curl(&self, args: &[String]) -> Result<(i32, String)> {
        // Common auth flags for all requests
        let mut command = Command::new("curl");
        
        // Add auth flags - these are the Amazon-specific ones
        command
            .arg("--location-trusted")
            .arg("--negotiate")
            .arg("-u")
            .arg(":")
            .arg("-s")  // Silent mode
            .arg("-w")
            .arg("%{stderr}%{http_code}")  // Write status code to stderr
            .arg("-b")
            .arg(self.home_dir.join(".midway/cookie").to_str().unwrap_or("/tmp/cookie"))
            .arg("-c")
            .arg(self.home_dir.join(".midway/cookie").to_str().unwrap_or("/tmp/cookie"))
            .args(args);
        
        // Execute the command
        let output = command.output()
            .map_err(|e| UVError::ExecutionError(format!("Failed to execute curl: {}", e)))?;
        
        // Get response body from stdout
        let body = String::from_utf8(output.stdout)
            .map_err(|e| UVError::ExecutionError(format!("Invalid UTF-8 in response: {}", e)))?;
            
        // Get status code from stderr (where -w output goes)
        let status_str = String::from_utf8(output.stderr)
            .map_err(|e| UVError::ExecutionError(format!("Invalid UTF-8 in status: {}", e)))?;
            
        let status: i32 = if status_str.is_empty() {
            // No status code returned, assume success
            200
        } else {
            status_str.trim().parse()
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse status code '{}': {}", status_str, e)))?
        };
        
        Ok((status, body))
    }
    
    /// Handle GET requests.
    fn handle_get(&self, id: Uuid, input: GetInput, link: &UVLink) -> Result<()> {
        let mut curl_args = self.get_headers_args(&input.headers);
        curl_args.push(input.url);
        
        // Execute the curl command
        let (status, body) = self.execute_curl(&curl_args)?;
        
        // Create and emit the response
        let response = HttpResponse { status, body };
        link.emit_photon(id, serde_json::to_value(response)?)?;
        
        // Signal successful completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle POST requests.
    fn handle_post(&self, id: Uuid, input: PostInput, link: &UVLink) -> Result<()> {
        let mut curl_args = Vec::new();
        
        // Add method if specified, default to POST
        let method = input.method.unwrap_or_else(|| "POST".to_string());
        curl_args.push("-X".to_string());
        curl_args.push(method);
        
        // Add headers
        curl_args.extend(self.get_headers_args(&input.headers));
        
        // Add body and URL
        curl_args.push("--data".to_string());
        curl_args.push(input.body);
        curl_args.push(input.url);
        
        // Execute the curl command
        let (status, body) = self.execute_curl(&curl_args)?;
        
        // Create and emit the response
        let response = HttpResponse { status, body };
        link.emit_photon(id, serde_json::to_value(response)?)?;
        
        // Signal successful completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
}

impl UVPrism for CurlPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "get" => {
                    // Deserialize the input
                    let input: GetInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the GET request
                    self.handle_get(id, input, link)?;
                    return Ok(true);
                },
                "post" => {
                    // Deserialize the input
                    let input: PostInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the POST request
                    self.handle_post(id, input, link)?;
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
        
        Ok(false)
    }
}

// Export a function to create a new instance
// This will be used by the dynamic loading system
#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(CurlPrism::new())
}
