use serde_json::Value;
use std::time::Duration;

use crate::spectrum::{ExecuteRequest, AgentPlan};
use uv_core::{
    Result, UVError, UVLink, UVSpectrum, UVPulse
};

/// Handles communication with the context prism and response collection
pub struct ContextCollector {
    spectrum: UVSpectrum,
}

impl ContextCollector {
    /// Create a new ContextCollector
    pub fn new(spectrum: UVSpectrum) -> Self {
        Self { spectrum }
    }

    /// Collect complete response from context prism
    pub async fn collect_response(&self, request: ExecuteRequest) -> Result<String> {
        // Get the context refraction
        let context_refraction = self.spectrum.find_refraction("context.chat")
            .ok_or_else(|| UVError::RefractionError("context.chat refraction not found".to_string()))?;

        // Create payload for context prism
        let mut context_payload = serde_json::json!({
            "prompt": request.prompt,
            "include_examples": request.include_examples
        });

        if let Some(model) = request.model {
            context_payload["model"] = Value::String(model);
        }

        // Create multiplexer and refract to context
        let multiplexer = uv_core::PrismMultiplexer::new();
        let context_link = multiplexer.refract(context_refraction, context_payload)?;

        // Collect all tokens until complete
        self.stream_tokens_until_complete(&context_link).await
    }

    /// Stream tokens from context link until complete, accumulating into full response
    async fn stream_tokens_until_complete(&self, link: &UVLink) -> Result<String> {
        let mut accumulated_response = String::new();

        loop {
            match link.receive()? {
                Some((_, UVPulse::Photon(photon))) => {
                    // Extract the token from the photon data
                    if let Some(token_obj) = photon.data.as_object() {
                        if let Some(token) = token_obj.get("token").and_then(|t| t.as_str()) {
                            accumulated_response.push_str(token);
                        }
                    }
                },
                Some((_, UVPulse::Trap(trap))) => {
                    // Check for errors
                    if let Some(error) = trap.error {
                        return Err(error);
                    }
                    // Success - we're done collecting
                    break;
                },
                Some((_, UVPulse::Extinguish)) => {
                    return Err(UVError::TransportError("Connection terminated".to_string()));
                },
                Some(_) => continue, // Ignore other pulse types
                None => {
                    // No message received, wait a bit
                    tokio::time::sleep(Duration::from_millis(10)).await;
                },
            }
        }

        Ok(accumulated_response)
    }

    /// Parse the accumulated response into an AgentPlan
    pub fn parse_agent_plan(&self, response: &str) -> Result<AgentPlan> {
        // The response should be a JSON object
        let plan: AgentPlan = serde_json::from_str(response)
            .map_err(|e| UVError::InvalidInput(format!("Failed to parse agent plan: {}", e)))?;

        Ok(plan)
    }

    /// Convenience method to collect and parse in one step
    pub async fn collect_and_parse(&self, request: ExecuteRequest) -> Result<AgentPlan> {
        let response = self.collect_response(request).await?;
        self.parse_agent_plan(&response)
    }
}
