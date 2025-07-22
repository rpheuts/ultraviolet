//! Ollama prism implementation for the Ultraviolet system.
//!
//! This crate provides an Ollama local AI model integration prism
//! that allows other prisms to access Ollama models via HTTP API.

pub mod spectrum;

use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::runtime::Runtime;
use uuid::Uuid;

use spectrum::{DEFAULT_MODEL, get_ollama_api_url, InvokeRequest};
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// Ollama prism for accessing Ollama local AI models via HTTP API.
pub struct OllamaPrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
    /// Tokio runtime for async operations
    runtime: Runtime,
    /// HTTP client for API requests
    client: Client,
}

impl OllamaPrism {
    /// Create a new Ollama prism.
    pub fn new() -> Result<Self> {
        // Create a tokio runtime
        let runtime = Runtime::new()
            .map_err(|e| UVError::ExecutionError(format!("Failed to create runtime: {}", e)))?;
        
        // Create HTTP client
        let client = Client::new();

        Ok(Self {
            spectrum: None,
            runtime,
            client,
        })  
    }

    /// Invoke a model with the given prompt (non-streaming)
    async fn invoke_model(
        &self,
        model: &str,
        prompt: &str,
        _max_tokens: i32,
    ) -> Result<(String, Option<i32>)> {
        // Build the request payload in Ollama chat format
        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "stream": false
        });

        let api_url = get_ollama_api_url();

        // Send the request
        let response = self.client
            .post(&api_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| UVError::ExecutionError(format!(
                "Failed to send request to Ollama ({}): {} - Make sure Ollama is running locally", 
                api_url, e
            )))?;

        // Check if the request was successful
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(UVError::ExecutionError(format!(
                "Ollama API error ({}): {} - Make sure the model '{}' is available in Ollama",
                status, error_text, model
            )));
        }

        // Parse the response
        let response_json: Value = response.json().await
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse Ollama response: {}", e)))?;

        // Extract the response text from Ollama's format
        let content = response_json
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| UVError::ExecutionError("Invalid response format from Ollama".to_string()))?;

        // Ollama doesn't typically return token counts in the same way, so we'll return None
        // In the future, we could estimate tokens or use a separate endpoint if available
        Ok((content.to_string(), None))
    }

    /// Invoke a model with streaming response
    async fn invoke_model_stream(
        &self,
        model: &str,
        prompt: &str,
        _max_tokens: i32,
        id: Uuid,
        link: &UVLink,
    ) -> Result<()> {
        // Build the request payload for streaming
        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "stream": true
        });

        let api_url = get_ollama_api_url();

        // Send the streaming request
        let response = self.client
            .post(&api_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| UVError::ExecutionError(format!(
                "Failed to send streaming request to Ollama ({}): {} - Make sure Ollama is running locally", 
                api_url, e
            )))?;

        // Check if the request was successful
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(UVError::ExecutionError(format!(
                "Ollama API error ({}): {} - Make sure the model '{}' is available in Ollama",
                status, error_text, model
            )));
        }

        // Process the streaming response
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| UVError::ExecutionError(format!("Failed to read stream chunk: {}", e)))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // Process complete lines from the buffer
            let mut lines_to_remove = Vec::new();
            let mut search_start = 0;
            
            while let Some(line_end) = buffer[search_start..].find('\n') {
                let actual_end = search_start + line_end;
                let line = buffer[search_start..actual_end].trim();
                lines_to_remove.push(actual_end + 1); // +1 to include the newline

                // Skip empty lines
                if line.is_empty() {
                    search_start = actual_end + 1;
                    continue;
                }

                // Parse the JSON data (Ollama sends one JSON object per line)
                match serde_json::from_str::<Value>(line) {
                    Ok(json_data) => {
                        // Check if this is the final message
                        if let Some(done) = json_data.get("done").and_then(|d| d.as_bool()) {
                            if done {
                                break;
                            }
                        }

                        // Extract token from message content
                        if let Some(content) = json_data
                            .get("message")
                            .and_then(|message| message.get("content"))
                            .and_then(|content| content.as_str())
                        {
                            if !content.is_empty() {
                                link.emit_photon(id, json!({"token": content}))?;
                            }
                        }
                    }
                    Err(_) => {
                        // Skip malformed JSON lines
                    }
                }
                
                search_start = actual_end + 1;
            }
            
            // Remove processed lines from buffer
            if let Some(&last_pos) = lines_to_remove.last() {
                buffer.drain(..last_pos);
            }
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

        // Get the model ID (or use default)
        let model = request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());

        // Invoke the model using the tokio runtime
        let (response, prompt_tokens) = self.runtime.block_on(async {
            self.invoke_model(&model, &request.prompt, request.max_tokens).await
        })?;

        // Build response with optional prompt tokens
        let mut response_json = json!({ "response": response });
        if let Some(tokens) = prompt_tokens {
            response_json["prompt_tokens"] = json!(tokens);
        }

        // Emit the response
        link.emit_photon(id, response_json)?;

        // Signal successful completion
        link.emit_trap(id, None)?;

        Ok(())
    }

    /// Handle 'invoke_stream' frequency
    fn handle_invoke_stream(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: InvokeRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        // Get the model ID (or use default)
        let model = request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());

        // Invoke the model with streaming using the tokio runtime
        self.runtime.block_on(async {
            self.invoke_model_stream(&model, &request.prompt, request.max_tokens, id, link).await
        })?;

        Ok(())
    }
}

impl UVPrism for OllamaPrism {
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
    Box::new(OllamaPrism::new().unwrap())
}
