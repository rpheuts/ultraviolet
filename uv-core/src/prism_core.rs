//! Core functionality for prisms in the Ultraviolet system.
//!
//! The PrismCore provides core functionality for prisms, including refraction handling
//! and the main processing loop.

use std::sync::Arc;
use serde_json::Value;

use crate::error::{UVError, Result};
use crate::link::UVLink;
use crate::prism::UVPrism;
use crate::pulse::UVPulse;
use crate::multiplexer::PrismMultiplexer;

/// Core functionality for prisms.
pub struct PrismCore {
    /// The prism being managed
    prism: Box<dyn UVPrism>,
    
    /// The multiplexer for managing prism connections
    multiplexer: Arc<PrismMultiplexer>,
    
    /// The link for communication with the prism
    link: Option<UVLink>,
}

impl PrismCore {
    /// Create a new PrismCore.
    pub fn new(prism: Box<dyn UVPrism>, multiplexer: Arc<PrismMultiplexer>) -> Self {
        Self {
            prism,
            multiplexer,
            link: None,
        }
    }
    
    /// Establish a link with the prism.
    pub async fn establish_link(&mut self, link: UVLink) -> Result<()> {
        // Store the link
        self.link = Some(link.clone());
        
        // Call the prism's link_established hook
        self.prism.link_established(&link).await?;
        
        Ok(())
    }
    
    /// Run the main processing loop.
    pub async fn attenuate(&self) {
        let link = match &self.link {
            Some(link) => link,
            None => return,
        };
        
        // Main processing loop
        while let Ok(Some((id, pulse))) = link.receive().await {
            match &pulse {
                UVPulse::Extinguish => {
                    // Let the prism handle the extinguish pulse first
                    let _ = self.prism.handle_pulse(id, &pulse, link).await;
                    
                    // Call the shutdown hook
                    if let Err(e) = self.prism.shutdown().await {
                        eprintln!("Error during prism shutdown: {}", e);
                    }
                    
                    break; // Exit the loop
                },
                _ => {
                    // For all other pulses, delegate to the prism
                    match self.prism.handle_pulse(id, &pulse, link).await {
                        Ok(true) => {
                            // Pulse was handled by the prism
                            continue;
                        },
                        Ok(false) => {
                            // Prism chose to ignore this pulse
                            // We could add default handling here if needed
                            continue;
                        },
                        Err(e) => {
                            // Error handling the pulse
                            if let UVPulse::Wavefront(_) = pulse {
                                // For wavefronts, send an error trap
                                let _ = link.emit_trap(id, Some(e)).await;
                            } else {
                                // For other pulses, just log the error
                                eprintln!("Error handling pulse: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Call a refraction and get a link for responses.
    pub async fn refract(&self, name: &str, payload: Value) -> Result<UVLink> {
        // Get the spectrum from the prism
        let spectrum = self.prism.spectrum();
        
        // Look up the refraction in the spectrum
        let refraction = spectrum.find_refraction(name)
            .ok_or_else(|| UVError::RefractionError(format!("Refraction not found: {}", name)))?;
        
        // Use the multiplexer to handle the refraction
        self.multiplexer.refract(refraction, payload).await
    }
    
    /// Call a refraction and absorb the result.
    pub async fn refract_and_absorb<T>(&self, name: &str, payload: Value) -> Result<T>
    where
        T: for<'de> serde::de::Deserialize<'de>,
    {
        // Call the refraction and get a link
        let link = self.refract(name, payload).await?;
        
        // Absorb the result
        link.absorb::<T>().await
    }
}
