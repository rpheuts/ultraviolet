//! Bedrock prism implementation for the Ultraviolet system.
//!
//! This crate provides an AWS Bedrock AI model integration prism
//! that allows other prisms to access AWS Bedrock models.

pub mod spectrum;

use aws_sdk_bedrockruntime::types::{ContentBlockDelta, Message};
use aws_sdk_bedrockruntime::types::{ContentBlock, ConversationRole, ConverseStreamOutput};
use serde_json::{json, Value};
use tokio::runtime::Runtime;
use uuid::Uuid;
use claude_tokenizer::count_tokens;

use spectrum::{DEFAULT_MODEL, DEFAULT_REGION, InvokeRequest};
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

    /// Invoke a model with the given prompt (non-streaming) using the Converse API
    async fn invoke_model(
        &self,
        model: &str,
        prompt: &str,
        _max_tokens: i32,
    ) -> Result<String> {
        // Build the message for Converse API
        let message = Message::builder()
            .role(ConversationRole::User)
            .content(ContentBlock::Text(prompt.to_string()))
            .build()
            .map_err(|e| {
                UVError::ExecutionError(format!(
                    "Failed to construct message: {}",
                    e
                ))
            })?;
        
        // Send the request using the converse API
        let response = self.client.converse()
            .model_id(model)
            .messages(message)
            .send()
            .await
            .map_err(|e| {
                UVError::ExecutionError(format!(
                    "Failed to invoke Bedrock model: {} (Make sure AWS credentials are configured in ~/.aws/credentials and have Bedrock permissions)",
                    e.into_service_error()
                ))
            })?;

        // Extract the response text from the Converse API response format
       Ok(response.output()
            .ok_or_else(|| UVError::ExecutionError("Unable to receive output from model".into()))?
            .as_message()
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse model ouput: {:?}", e).into()))?
            .content()
            .iter().map(|v| {
                match v {
                    ContentBlock::ReasoningContent(reasoning_content_block) => {
                        match reasoning_content_block {
                            aws_sdk_bedrockruntime::types::ReasoningContentBlock::ReasoningText(reasoning_text_block) => reasoning_text_block.text.clone(),
                            _ => "".into(),
                        }
                    },
                    ContentBlock::Text(text) => text.clone(),
                    _ => "".into(),
                }
            })
            .collect::<Vec<String>>()
            .join(""))
    }

    /// Invoke a model with streaming response using the Converse API
    async fn invoke_model_stream(
        &self,
        model: &str,
        prompt: &str,
        _max_tokens: i32,
        id: Uuid,
        link: &UVLink,
    ) -> Result<()> {
        // Build the message for Converse API
        let message = Message::builder()
            .role(ConversationRole::User)
            .content(ContentBlock::Text(prompt.to_string()))
            .build()
            .map_err(|e| {
                UVError::ExecutionError(format!(
                    "Failed to construct message: {}",
                    e
                ))
            })?;

        let mut stream = self.client.converse_stream()
            .model_id(model)
            .messages(message)
            .send()
            .await
            .map_err(|e| {
                UVError::ExecutionError(format!(
                    "Failed to invoke Bedrock model with streaming: {} (Make sure AWS credentials are configured in ~/.aws/credentials and have Bedrock permissions)",
                    e
                ))
            })?;

            let mut reasoning = false;
        
            loop {
                match stream.stream.recv().await {
                    Ok(Some(event)) => {                        
                        match event {
                            ConverseStreamOutput::ContentBlockDelta(content_block_delta_event) => {
                                if let Some(delta) = content_block_delta_event.delta() {
                                    match delta {
                                        ContentBlockDelta::ReasoningContent(reasoning_content_block) => {
                                            match reasoning_content_block {
                                                aws_sdk_bedrockruntime::types::ReasoningContentBlockDelta::Text(reasoning_text_block) => {
                                                    if !reasoning {
                                                        link.emit_photon(id, json!({"token": "<reasoning>\n"}))?;
                                                        reasoning = true;
                                                    }

                                                    link.emit_photon(id, json!({"token": reasoning_text_block.clone()}))?
                                                },
                                                _ => continue,
                                            }
                                        },
                                        ContentBlockDelta::Text(text) => {
                                            if reasoning {
                                                link.emit_photon(id, json!({"token": "</reasoning>\n"}))?;
                                                reasoning = false;
                                            }

                                            link.emit_photon(id, json!({"token": text.clone()}))?
                                        },
                                        _ => continue,
                                    }    
                                }
                            },
                            ConverseStreamOutput::MessageStop(_) => break,
                            _ => continue,
                        }
                    },
                    Ok(None) => continue,
                    Err(e) => {
                        println!("Error receiving from stream: {:?}", e);
                        return Err(UVError::ExecutionError(format!("Failed to receive stream chunk: {}", e)));
                    }
                }
            }

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

        // Calculate prompt tokens before API call
        let prompt_tokens = count_tokens(&request.prompt)
            .map_err(|e| UVError::ExecutionError(format!("Failed to calculate prompt tokens: {}", e)))?;

        // Invoke the model using the tokio runtime
        let response = self.runtime.block_on(async {
            self.invoke_model(&model, &request.prompt, request.max_tokens).await
        })?;

        // Emit the response with prompt token count
        link.emit_photon(id, json!({ 
            "response": response,
            "usage": {
                "prompt_tokens": prompt_tokens,
                "source": "calculated"
            }
        }))?;

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

        // Calculate prompt tokens before API call
        let prompt_tokens = count_tokens(&request.prompt)
            .map_err(|e| UVError::ExecutionError(format!("Failed to calculate prompt tokens: {}", e)))?;

        // Send final token with usage information
        link.emit_photon(id, json!({
            "token": "",
            "usage": {
                "prompt_tokens": prompt_tokens,
                "source": "calculated"
            }
        }))?;

        // Invoke the model with streaming using the tokio runtime
        self.runtime.block_on(async {
            self.invoke_model_stream(&model, &request.prompt, request.max_tokens, id, link).await
        })?;

        // The invoke_model_stream function already sends the trap

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
