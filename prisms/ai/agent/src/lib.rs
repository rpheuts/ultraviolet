use std::sync::Arc;
use uuid::Uuid;
use serde_json::Value;

use uv_core::{Result, UVError, UVLink, UVPrism, UVSpectrum, UVPulse};

mod spectrum;
mod context_collector;
mod action_executor;
mod agent_flow;

use spectrum::{ExecuteRequest, AgentEvent};
use agent_flow::AgentFlow;

/// AI Agent prism that orchestrates planning and execution workflows
pub struct AIAgentPrism {
    spectrum: Option<Arc<UVSpectrum>>,
    agent_flow: Option<AgentFlow>,
}

impl AIAgentPrism {
    /// Create a new AI Agent prism
    pub fn new() -> Self {
        Self {
            spectrum: None,
            agent_flow: None,
        }
    }

    /// Handle the execute frequency
    async fn handle_execute(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let agent_flow = self.agent_flow.as_ref()
            .ok_or_else(|| UVError::ExecutionError("Agent flow not initialized".to_string()))?;

        // Parse the input request
        let request: ExecuteRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid execute request: {}", e)))?;

        // Execute the iterative agentic workflow
        agent_flow.execute_iterative_workflow(request, id, link).await
    }
}

impl UVPrism for AIAgentPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        let spectrum_arc = Arc::new(spectrum);
        
        // Clone the spectrum for the agent flow
        let spectrum_for_flow = (*spectrum_arc).clone();
        
        // Initialize components
        self.spectrum = Some(spectrum_arc);
        self.agent_flow = Some(AgentFlow::new(spectrum_for_flow));
        
        Ok(())
    }

    fn link_established(&mut self, _link: &UVLink) -> Result<()> {
        // No special setup needed for link establishment
        Ok(())
    }

    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        match pulse {
            UVPulse::Wavefront(wavefront) => {
                // Create a tokio runtime for async execution
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| UVError::ExecutionError(format!("Failed to create async runtime: {}", e)))?;

                match wavefront.frequency.as_str() {
                    "execute" => {
                        rt.block_on(async {
                            if let Err(e) = self.handle_execute(id, wavefront.input.clone(), link).await {
                                // Send error as an action output event and then trap
                                let error_event = AgentEvent::ActionOutput {
                                    action: spectrum::AgentAction {
                                        prism: "ai:agent".to_string(),
                                        frequency: "execute".to_string(),
                                        input: serde_json::Value::Null,
                                        description: "Internal error".to_string(),
                                    },
                                    success: false,
                                    data: None,
                                    error: Some(e.to_string()),
                                };
                                
                                if let Ok(error_json) = serde_json::to_value(&error_event) {
                                    let _ = link.emit_photon(id, error_json);
                                }
                                
                                let _ = link.emit_trap(id, Some(e));
                            }
                        });
                        Ok(true)
                    },
                    _ => {
                        let error = UVError::InvalidInput(format!("Unknown frequency: {}", wavefront.frequency));
                        link.emit_trap(id, Some(error))?;
                        Ok(true)
                    }
                }
            },
            _ => Ok(false), // Let other pulse types be handled elsewhere
        }
    }
}

/// Create a new AI Agent prism instance
#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(AIAgentPrism::new())
}
