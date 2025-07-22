//! Timer prism implementation for the Ultraviolet system.
//!
//! This crate provides a timer prism that allows workflows to wait for
//! specified durations before continuing execution.

pub mod spectrum;

use std::time::Duration;
use serde_json::{json, Value};
use tokio::runtime::Runtime;
use uuid::Uuid;

use spectrum::WaitRequest;
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// Timer prism for delays and scheduling in workflows
pub struct TimerPrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
    /// Tokio runtime for async operations
    runtime: Runtime,
}

impl TimerPrism {
    /// Create a new Timer prism
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new()
            .map_err(|e| UVError::ExecutionError(format!("Failed to create runtime: {}", e)))?;

        Ok(Self {
            spectrum: None,
            runtime,
        })
    }

    /// Handle the wait frequency
    async fn handle_wait(&self, duration_ms: u64, message: Option<String>, id: Uuid, link: &UVLink) -> Result<()> {
        // Emit optional message photon
        if let Some(msg) = message {
            link.emit_photon(id, json!({"message": msg}))?;
        }
        
        // Wait for specified duration
        tokio::time::sleep(Duration::from_millis(duration_ms)).await;
        
        // Complete successfully
        link.emit_trap(id, None)?;
        Ok(())
    }

    /// Handle 'wait' frequency
    fn handle_wait_sync(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: WaitRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        self.runtime.block_on(async {
            self.handle_wait(request.duration_ms, request.message, id, link).await
        })
    }
}

impl UVPrism for TimerPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }

    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "wait" => {
                    self.handle_wait_sync(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                _ => {
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(TimerPrism::new().unwrap())
}