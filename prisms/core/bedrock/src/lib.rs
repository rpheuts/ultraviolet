//! Bedrock prism implementation for the Ultraviolet system.
//!
//! This crate provides an AWS Bedrock AI model integration prism
//! that allows other prisms to access AWS Bedrock models.

pub mod spectrum;

use aws_sdk_bedrockruntime::primitives::Blob;
use aws_sdk_bedrockruntime::types::ResponseStream;
use serde_json::{json, Value};
use std::fs;
use tokio::runtime::Runtime;
use uuid::Uuid;

use spectrum::{DEFAULT_MODEL, DEFAULT_REGION, InvokeRequest, InvokeWithFilesRequest};
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// Bedrock prism for accessing AWS Bedrock AI models.
pub struct BedrockPrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
    /// Tokio runtime for async operations
    runtime: Runtime,
    /// AWS Bedrock client
    client: aws_sdk_bedrockruntime::Client,
}

impl BedrockPrism {
    /// Create a new Bedrock prism.
    pub fn new() -> Result<Self> {
        // Create a tokio runtime
        let runtime = Runtime::new()
            .map_err(|e| UVError::ExecutionError(format!("Failed to create runtime: {}", e)))?;
        
        // Initialize AWS client
        let client = runtime.block_on(async {
            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(DEFAULT_REGION))
                .load()
                .await;

            let client = aws_sdk_bedrockruntime::Client::new(&config);
            Ok::<aws_sdk_bedrockruntime::Client, UVError>(client)
        })?;

        Ok(Self {
            spectrum: None,
            runtime,
            client,
        })
    }

    /// Build a prompt with optional file context
    fn build_prompt(&self, prompt: &str, files: &[String]) -> Result<String> {
        let mut full_prompt = prompt.to_string();

        if !files.is_empty() {
            full_prompt.push_str("\n\nI'm also providing the contents of the following files for context:\n");
            
            for file in files {
                let content = fs::read_to_string(file)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to read file {}: {}", file, e)))?;
                
                full_prompt.push_str(&format!("\n=== File: {} ===\n{}\n", file, content));
            }

            full_prompt.push_str("\nPlease consider these files in your response.");
        }

        Ok(full_prompt)
    }

    /// Invoke a model with the given prompt (non-streaming)
    async fn invoke_model(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: i32,
    ) -> Result<String> {
        let request = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": max_tokens,
            "messages": [{
                "role": "user",
                "content": [{"type": "text", "text": prompt}]
            }]
        });

        let response = self.client.invoke_model()
            .model_id(model)
            .body(Blob::new(request.to_string()))
            .content_type("application/json")
            .accept("application/json")
            .send()
            .await
            .map_err(|e| {
                UVError::ExecutionError(format!(
                    "Failed to invoke Bedrock model: {} (Make sure AWS credentials are configured in ~/.aws/credentials and have Bedrock permissions)",
                    e
                ))
            })?;

        let response_json: Value = serde_json::from_slice(response.body.as_ref())
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse response: {}", e)))?;
        
        response_json["content"][0]["text"].as_str()
            .map(String::from)
            .ok_or_else(|| UVError::ExecutionError("Invalid response format".into()))
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
        let request = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": max_tokens,
            "messages": [{
                "role": "user",
                "content": [{"type": "text", "text": prompt}]
            }]
        });

        let mut stream = self.client.invoke_model_with_response_stream()
            .model_id(model)
            .body(Blob::new(request.to_string()))
            .content_type("application/json")
            .accept("application/json")
            .send()
            .await
            .map_err(|e| {
                UVError::ExecutionError(format!(
                    "Failed to invoke Bedrock model with streaming: {} (Make sure AWS credentials are configured in ~/.aws/credentials and have Bedrock permissions)",
                    e
                ))
            })?
            .body;

        // Process the streaming response
        while let Some(event) = stream.recv().await
            .map_err(|e| UVError::ExecutionError(format!("Failed to receive stream chunk: {}", e)))? {
            if let ResponseStream::Chunk(data) = event {
                let bytes = data.bytes
                    .ok_or_else(|| UVError::ExecutionError("Missing chunk bytes".into()))?;
                let chunk: Value = serde_json::from_slice(bytes.as_ref())
                    .map_err(|e| UVError::ExecutionError(format!("Failed to parse chunk: {}", e)))?;
                
                if let Some(text) = chunk["delta"]["text"].as_str() {
                    // Emit each chunk as a photon
                    link.emit_photon(id, json!({"token": text}))?;
                }
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
        let response = self.runtime.block_on(async {
            self.invoke_model(&model, &request.prompt, request.max_tokens).await
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

        // Get the model ID (or use default)
        let model = request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());

        // Invoke the model with streaming using the tokio runtime
        self.runtime.block_on(async {
            self.invoke_model_stream(&model, &request.prompt, request.max_tokens, id, link).await
        })?;

        // The invoke_model_stream function already sends the trap

        Ok(())
    }

    /// Handle 'invoke_with_files' frequency
    fn handle_invoke_with_files(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: InvokeWithFilesRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        // Build the prompt with file context
        let full_prompt = self.build_prompt(&request.prompt, &request.files)?;

        // Get the model ID (or use default)
        let model = request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());

        // Invoke the model using the tokio runtime
        let response = self.runtime.block_on(async {
            self.invoke_model(&model, &full_prompt, request.max_tokens).await
        })?;

        // Emit the response
        link.emit_photon(id, json!({ "response": response }))?;

        // Signal successful completion
        link.emit_trap(id, None)?;

        Ok(())
    }
}

impl UVPrism for BedrockPrism {
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
                "invoke_with_files" => {
                    self.handle_invoke_with_files(id, wavefront.input.clone(), link)?;
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
    Box::new(BedrockPrism::new().unwrap())
}