use std::collections::HashMap;

use crate::spectrum::{AgentAction, ActionResult};
use uv_core::{
    Result, UVError, UVLink, PrismMultiplexer, UVPulse
};
use uv_core::Refraction;

/// Handles execution of actions against UV prisms
pub struct ActionExecutor {
    multiplexer: PrismMultiplexer,
}

impl ActionExecutor {
    /// Create a new ActionExecutor
    pub fn new() -> Self {
        Self {
            multiplexer: PrismMultiplexer::new(),
        }
    }

    /// Execute a single action
    pub async fn execute_action(&self, action: &AgentAction) -> Result<ActionResult> {
        match self.execute_action_internal(action).await {
            Ok(result) => Ok(ActionResult {
                action: action.clone(),
                success: true,
                result: Some(result),
                error: None,
            }),
            Err(error) => Ok(ActionResult {
                action: action.clone(),
                success: false,
                result: None,
                error: Some(error.to_string()),
            }),
        }
    }

    /// Internal method to execute action and return result or error
    async fn execute_action_internal(&self, action: &AgentAction) -> Result<serde_json::Value> {
        // Create a dynamic refraction for this action
        let refraction = self.create_dynamic_refraction(action)?;
        
        // Execute the refraction
        let link = self.multiplexer.refract(&refraction, action.input.clone())?;
        
        // Collect the result
        self.collect_action_result(&link).await
    }

    /// Create a dynamic refraction for the given action
    fn create_dynamic_refraction(&self, action: &AgentAction) -> Result<Refraction> {
        // For now, we'll create a simple pass-through refraction
        // This assumes the input format matches what the target prism expects
        let mut transpose = HashMap::new();
        if let Some(input_obj) = action.input.as_object() {
            for key in input_obj.keys() {
                transpose.insert(key.clone(), key.clone());
            }
        }

        Ok(Refraction {
            name: format!("{}.{}", action.prism, action.frequency),
            target: action.prism.clone(),
            frequency: action.frequency.clone(),
            transpose: transpose, // Pass through input as-is
            reflection: HashMap::new(), // Accept all outputs
        })
    }

    /// Collect result from action execution
    async fn collect_action_result(&self, link: &UVLink) -> Result<serde_json::Value> {
        let mut data = Vec::new();

        // Collect all photons until we get a trap
        loop {
            match link.receive()? {
                Some((_, UVPulse::Photon(photon))) => {
                    data.push(photon.data);
                },
                Some((_, UVPulse::Trap(trap))) => {
                    // If there's an error, return it
                    if let Some(error) = trap.error {
                        return Err(error);
                    }
                    // Otherwise, we're done collecting
                    break;
                },
                Some((_, UVPulse::Extinguish)) => {
                    return Err(UVError::TransportError("Connection terminated".to_string()));
                },
                Some(_) => continue, // Ignore other pulse types
                None => {
                    // No message received, wait a bit
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                },
            }
        }

        // If we have multiple photons, combine them into an array
        let result = if data.len() > 1 {
            serde_json::to_value(data)?
        } else if data.len() == 1 {
            data.into_iter().next().unwrap()
        } else {
            // No data received, return empty object
            serde_json::json!({})
        };

        Ok(result)
    }
}
