//! OpenRouter prism implementation for the Ultraviolet system.
//!
//! This crate provides an OpenRouter AI model integration prism
//! that allows other prisms to access OpenRouter models via HTTP API.

pub mod spectrum;

use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use tokio::runtime::Runtime;
use uuid::Uuid;

use spectrum::{DEFAULT_MODEL, OPENROUTER_API_URL, InvokeRequest};
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// OpenRouter prism for accessing OpenRouter AI models via HTTP API.
pub struct OpenRouterPrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
    /// Tokio runtime for async operations
    runtime: Runtime,
    /// HTTP client for API requests
    client: Client,
    /// OpenRouter API key
    api_key: String,
}

impl OpenRouterPrism {
    /// Create a new OpenRouter prism.
    pub fn new() -> Result<Self> {
        // Create a tokio runtime
        let runtime = Runtime::new()
            .map_err(|e| UVError::ExecutionError(format!("Failed to create runtime: {}", e)))?;
        
        // Get API key from environment
        let api_key = env::var("OPENROUTER_API_KEY")
            .map_err(|_| UVError::ExecutionError(
                "OpenRouter API key not found. Set OPENROUTER_API_KEY environment variable.".to_string()
            ))?;

        // Create HTTP client
        let client = Client::new();

        Ok(Self {
            spectrum: None,
            runtime,
            client,
            api_key,
        })  
    }

    /// Invoke a model with the given prompt (non-streaming)
    async fn invoke_model(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: i32,
    ) -> Result<(String, Option<i32>)> {
        // Build the request payload in OpenAI chat completions format
        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": max_tokens,
            "stream": false
        });

        // Send the request
        let response = self.client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/ultraviolet-system")
            .header("X-Title", "Ultraviolet System")
            .json(&payload)
            .send()
            .await
            .map_err(|e| UVError::ExecutionError(format!("Failed to send request to OpenRouter: {}", e)))?;

        // Check if the request was successful
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(UVError::ExecutionError(format!(
                "OpenRouter API error ({}): {}",
                status, error_text
            )));
        }

        // Parse the response
        let response_json: Value = response.json().await
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse OpenRouter response: {}", e)))?;

        // Extract the response text
        let content = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| UVError::ExecutionError("Invalid response format from OpenRouter".to_string()))?;

        // Extract usage information if available
        let prompt_tokens = response_json
            .get("usage")
            .and_then(|usage| usage.get("prompt_tokens"))
            .and_then(|tokens| tokens.as_i64())
            .map(|tokens| tokens as i32);

        Ok((content.to_string(), prompt_tokens))
    }

    /// Invoke a model with streaming response
    async fn invoke_model_stream(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: i32,
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
            "max_tokens": max_tokens,
            "stream": true
        });

        // Send the streaming request
        let response = self.client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/ultraviolet-system")
            .header("X-Title", "Ultraviolet System")
            .json(&payload)
            .send()
            .await
            .map_err(|e| UVError::ExecutionError(format!("Failed to send streaming request to OpenRouter: {}", e)))?;

        // Check if the request was successful
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(UVError::ExecutionError(format!(
                "OpenRouter API error ({}): {}",
                status, error_text
            )));
        }

        // Process the streaming response
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut prompt_tokens: Option<i32> = None;

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

                // Skip empty lines and comments
                if line.is_empty() || !line.starts_with("data: ") {
                    search_start = actual_end + 1;
                    continue;
                }

                let data = &line[6..]; // Remove "data: " prefix

                // Check for end of stream
                if data == "[DONE]" {
                    break;
                }

                // Parse the JSON data
                match serde_json::from_str::<Value>(data) {
                    Ok(json_data) => {
                        // Extract token from delta
                        if let Some(content) = json_data
                            .get("choices")
                            .and_then(|choices| choices.get(0))
                            .and_then(|choice| choice.get("delta"))
                            .and_then(|delta| delta.get("content"))
                            .and_then(|content| content.as_str())
                        {
                            if !content.is_empty() {
                                link.emit_photon(id, json!({"token": content}))?;
                            }
                        }

                        // Extract usage information if available
                        if prompt_tokens.is_none() {
                            if let Some(usage_tokens) = json_data
                                .get("usage")
                                .and_then(|usage| usage.get("prompt_tokens"))
                                .and_then(|tokens| tokens.as_i64())
                            {
                                prompt_tokens = Some(usage_tokens as i32);
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

        // Send final token with usage information if available
        if let Some(tokens) = prompt_tokens {
            link.emit_photon(id, json!({
                "token": "",
                "prompt_tokens": tokens
            }))?;
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

impl UVPrism for OpenRouterPrism {
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
    Box::new(OpenRouterPrism::new().unwrap())
}
