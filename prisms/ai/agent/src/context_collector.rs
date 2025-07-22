use serde_json::Value;
use uuid::Uuid;
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

    /// Stream tokens and forward usage data from context link
    pub async fn stream_with_usage_forwarding(&self, request: ExecuteRequest, id: Uuid, link: &UVLink) -> Result<String> {
        // Get the context refraction
        let context_refraction = self.spectrum.find_refraction("context.chat")
            .ok_or_else(|| UVError::RefractionError("context.chat refraction not found".to_string()))?;

        // Create payload for context prism
        let mut context_payload = serde_json::json!({
            "prompt": request.prompt,
            "include_examples": request.include_examples,
            "backend": request.backend
        });

        if let Some(model) = request.model {
            context_payload["model"] = Value::String(model);
        }

        // Create multiplexer and refract to context
        let multiplexer = uv_core::PrismMultiplexer::new();
        let context_link = multiplexer.refract(context_refraction, context_payload)?;

        let mut accumulated_response = String::new();

        loop {
            match context_link.receive()? {
                Some((_, UVPulse::Photon(photon))) => {
                    // Forward the photon to the agent's output (preserves usage data)
                    link.emit_photon(id, photon.data.clone())?;
                    
                    // Also extract tokens for our own processing
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

    /// Extract JSON from Q CLI response that may contain text before the JSON
    fn extract_json_from_response(response: &str) -> String {
        // Look for JSON object starting with { and ending with }
        if let Some(start) = response.find('{') {
            // Find the matching closing brace
            let mut brace_count = 0;
            let mut end_pos = start;
            
            for (i, ch) in response[start..].char_indices() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            end_pos = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            
            if brace_count == 0 {
                let json_str = &response[start..end_pos];
                // Validate it's actually JSON by trying to parse it
                if serde_json::from_str::<Value>(json_str).is_ok() {
                    return json_str.to_string();
                }
            }
        }
        
        // If no valid JSON found, return the original response
        response.to_string()
    }

    /// Parse the accumulated response into an AgentPlan
    pub fn parse_agent_plan(&self, response: &str) -> Result<AgentPlan> {
        // The issue is that the response contains unescaped newlines that break JSON structure
        // We need to reconstruct this as proper JSON

        // Check if response is empty (likely due to upstream error)
        if response.trim().is_empty() {
            return Err(UVError::InvalidInput(
                "Empty response from AI model - this may indicate an upstream error (check Q CLI status)".to_string()
            ));
        }

        // Extract JSON from the response
        let json_response = Self::extract_json_from_response(&response);
        
        // Try to parse as-is first, if it fails, attempt to fix the JSON structure
        match serde_json::from_str::<AgentPlan>(&json_response) {
            Ok(plan) => Ok(plan),
            Err(parse_error) => {
                // JSON is malformed, likely due to unescaped newlines
                // Try to fix by joining all lines and re-escaping
                let single_line = json_response.replace('\n', "").replace('\r', "");
                
                match serde_json::from_str::<AgentPlan>(&single_line) {
                    Ok(plan) => Ok(plan),
                    Err(_) => {
                        // Both attempts failed - provide helpful error with context
                        let truncated_response = if response.len() > 200 {
                            format!("{}...", &response[..200])
                        } else {
                            response.to_string()
                        };
                        
                        Err(UVError::InvalidInput(format!(
                            "Failed to parse AI response as JSON: {} | Response received: '{}'", 
                            parse_error, 
                            truncated_response
                        )))
                    }
                }
            }
        }
    }
}
