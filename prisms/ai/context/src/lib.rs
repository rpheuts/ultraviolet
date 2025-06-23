//! AI Context prism implementation for the Ultraviolet system.
//!
//! This crate provides an AI context enrichment prism that takes user prompts,
//! enriches them with UV system context, and streams AI responses.

pub mod spectrum;

use std::time::Duration;

use serde_json::{json, Value};
use tokio::runtime::Runtime;
use uuid::Uuid;
use regex::Regex;

use spectrum::{ChatRequest, PrismInfo, PrismSpectrum, PROMPT_TEMPLATE};
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// AI Context prism for enriching prompts with UV system context.
pub struct AIContextPrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
    /// Tokio runtime for async operations
    runtime: Runtime,
}

impl AIContextPrism {
    /// Create a new AI Context prism.
    pub fn new() -> Result<Self> {
        // Create a tokio runtime
        let runtime = Runtime::new()
            .map_err(|e| UVError::ExecutionError(format!("Failed to create runtime: {}", e)))?;

        Ok(Self {
            spectrum: None,
            runtime,
        })
    }

    /// Get available prisms from the discovery service
    async fn get_available_prisms(&self, filter: &Option<String>) -> Result<Vec<PrismInfo>> {
        // Use refraction to get prisms from discovery
        let spectrum = self.spectrum.as_ref()
            .ok_or_else(|| UVError::ExecutionError("Prism not initialized".to_string()))?;
        
        let discovery_refraction = spectrum.find_refraction("discovery.list")
            .ok_or_else(|| UVError::RefractionError("discovery.list refraction not found".to_string()))?;
        
        // Create multiplexer and refract to discovery
        let multiplexer = uv_core::PrismMultiplexer::new();
        let payload = match filter {
            Some(f) => json!({"filter": f}),
            None => json!({})
        };
        
        let link = multiplexer.refract(discovery_refraction, payload)?;
        let result: Vec<PrismInfo> = link.absorb()?;
        
        // Apply regex filter if provided
        if let Some(filter_str) = filter {
            if let Ok(regex) = Regex::new(filter_str) {
                return Ok(result.into_iter()
                    .filter(|p| regex.is_match(&format!("{}:{}", p.namespace, p.name)))
                    .collect());
            }
        }
        
        Ok(result)
    }

    /// Get detailed spectrum for a specific prism
    async fn get_prism_spectrum(&self, prism_id: &str) -> Result<PrismSpectrum> {
        let spectrum = self.spectrum.as_ref()
            .ok_or_else(|| UVError::ExecutionError("Prism not initialized".to_string()))?;
        
        let discovery_refraction = spectrum.find_refraction("discovery.describe")
            .ok_or_else(|| UVError::RefractionError("discovery.describe refraction not found".to_string()))?;
        
        let multiplexer = uv_core::PrismMultiplexer::new();
        let payload = json!({"prismId": prism_id});
        
        let link = multiplexer.refract(discovery_refraction, payload)?;
        let result: PrismSpectrum = link.absorb()?;
        Ok(result)
    }

    /// Build enriched prompt with UV system context
    async fn build_enriched_prompt(&self, user_prompt: &str, prisms: &[PrismInfo], include_examples: bool) -> Result<String> {
        let mut capabilities = String::new();
        
        for prism in prisms {
            let prism_id = format!("{}:{}", prism.namespace, prism.name);
            
            // Get detailed spectrum for this prism
            match self.get_prism_spectrum(&prism_id).await {
                Ok(spectrum) => {
                    capabilities.push_str(&format!("### {}\n", prism_id));
                    capabilities.push_str(&format!("{}\n\n", spectrum.description));
                    
                    capabilities.push_str("**Available frequencies:**\n");
                    for wavelength in &spectrum.wavelengths {
                        capabilities.push_str(&format!("- `{}`: {}\n", wavelength.frequency, wavelength.description));
                        
                        if include_examples {
                            let example = format!("uv {} {} '{{\"param\": \"value\"}}'", 
                                prism_id, wavelength.frequency);
                            capabilities.push_str(&format!("  Example: `{}`\n", example));
                        }
                    }
                    capabilities.push_str("\n");
                },
                Err(_) => {
                    // Skip prisms we can't describe
                    continue;
                }
            }
        }
        
        Ok(PROMPT_TEMPLATE
            .replace("{prism_capabilities}", &capabilities)
            .replace("{user_prompt}", user_prompt))
    }

    /// Stream response from bedrock
    async fn stream_from_bedrock(&self, id: Uuid, prompt: String, model: Option<String>, link: &UVLink) -> Result<()> {
        let spectrum = self.spectrum.as_ref()
            .ok_or_else(|| UVError::ExecutionError("Prism not initialized".to_string()))?;
        
        let bedrock_refraction = spectrum.find_refraction("bedrock.invoke_stream")
            .ok_or_else(|| UVError::RefractionError("bedrock.invoke_stream refraction not found".to_string()))?;
        
        // Create payload for bedrock
        let mut bedrock_payload = json!({
            "prompt": prompt,
            "max_tokens": 4096
        });
        
        if let Some(m) = model {
            bedrock_payload["model"] = json!(m);
        }
        
        // Create multiplexer and refract to bedrock
        let multiplexer = uv_core::PrismMultiplexer::new();
        let bedrock_link = multiplexer.refract(bedrock_refraction, bedrock_payload)?;

        loop {
            match bedrock_link.receive()? {
                Some((_, UVPulse::Photon(photon))) => {
                    link.emit_photon(id, photon.data)?;
                },
                Some((_, UVPulse::Trap(trap))) => {
                    link.emit_trap(id, trap.error)?;
                    break;
                },
                Some((_, UVPulse::Extinguish)) => {
                    return Err(UVError::TransportError("Connection terminated".to_string()));
                },
                Some(_) => continue, // Ignore other pulse types
                None => {
                    // No message received, wait a bit
                    std::thread::sleep(Duration::from_millis(10));
                },
            }
        }

        Ok(())
    }

    /// Handle 'chat' frequency
    fn handle_chat(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: ChatRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        // Process the chat request using the tokio runtime
        self.runtime.block_on(async {
            // Get available prisms from discovery
            let prisms = self.get_available_prisms(&request.prism_filter).await?;
            
            // Build enriched prompt
            let enriched_prompt = self.build_enriched_prompt(
                &request.prompt, 
                &prisms, 
                request.include_examples
            ).await?;
            
            // Stream from bedrock
            self.stream_from_bedrock(id, enriched_prompt, request.model, link).await?;
            
            Ok::<(), UVError>(())
        })?;

        Ok(())
    }
}

impl UVPrism for AIContextPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "chat" => {
                    self.handle_chat(id, wavefront.input.clone(), link)?;
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
    Box::new(AIContextPrism::new().unwrap())
}
