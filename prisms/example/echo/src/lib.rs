//! Echo prism implementation for the Ultraviolet system.
//!
//! This crate provides a simple echo prism that reflects back any input it receives.
//! It demonstrates how to implement a prism as a standalone crate.

pub mod spectrum;

use std::sync::Arc;
use uuid::Uuid;

use uv_core::{
    Result, UVLink, UVPrism, PrismMultiplexer, UVPulse, UVSpectrum
};

/// Echo prism that reflects back any input it receives.
pub struct EchoPrism {
    spectrum: Option<UVSpectrum>,
}

impl EchoPrism {
    /// Create a new echo prism.
    pub fn new() -> Self {
        Self { spectrum: None }
    }
}

impl UVPrism for EchoPrism {
    fn init_spectrum(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn init_multiplexer(&mut self, _multiplexer: Arc<PrismMultiplexer>) -> Result<()> {
        // Echo prism doesn't need the multiplexer
        Ok(())
    }
    
    fn spectrum(&self) -> &UVSpectrum {
        self.spectrum.as_ref().expect("Prism not initialized")
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            if wavefront.frequency == "echo" {
                // Deserialize the input
                let input: spectrum::EchoInput = serde_json::from_value(wavefront.input.clone())?;
                
                // Log the received input
                println!("Echo prism received: {:?}", &input.message);
                
                // Create the output
                let output = spectrum::EchoOutput {
                    message: input.message.clone(),
                };
                
                // Send the response
                link.reflect(id, output)?;
                return Ok(true);
            }
        }
        
        Ok(false)
    }
}

// Export a function to create a new instance
// This will be used by the dynamic loading system
#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(EchoPrism::new())
}
