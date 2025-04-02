//! Core functionality for prisms in the Ultraviolet system.
//!
//! The PrismCore provides core functionality for prisms, including refraction handling
//! and the main processing loop.


use crate::error::Result;
use crate::link::UVLink;
use crate::prism::UVPrism;
use crate::pulse::UVPulse;

/// Core functionality for prisms.
pub struct UVPrismCore {
    /// The prism being managed
    prism: Box<dyn UVPrism>,
}

impl UVPrismCore {
    /// Create a new PrismCore.
    pub fn new(prism: Box<dyn UVPrism>) -> Self {
        Self {
            prism,
        }
    }
    
    /// Run the main processing loop.
    /// 
    /// This function runs the main loop for the prism, receiving pulses from the link
    /// and delegating them to the prism's handle_pulse method. It continues until
    /// an Extinguish pulse is received or the link is closed.
    pub fn run_loop(&self, link: UVLink) -> Result<()> {
        // Main processing loop
        loop {
            match link.receive() {
                Ok(Some((id, pulse))) => {
                    match pulse {
                        UVPulse::Extinguish => {
                            // Let the prism handle the extinguish pulse first
                            let _ = self.prism.handle_pulse(id, &UVPulse::Extinguish, &link);
                            
                            // Call the shutdown hook
                            if let Err(e) = self.prism.shutdown() {
                                eprintln!("Error during prism shutdown: {}", e);
                            }
                            
                            break; // Exit the loop
                        },
                        ref other_pulse => {
                            // For all other pulses, delegate to the prism
                            match self.prism.handle_pulse(id, other_pulse, &link) {
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
                                    if let UVPulse::Wavefront(_) = other_pulse {
                                        // For wavefronts, send an error trap
                                        let _ = link.emit_trap(id, Some(e));
                                    } else {
                                        // For other pulses, just log the error
                                        eprintln!("Error handling pulse: {}", e);
                                    }
                                }
                            }
                        }
                    }
                },
                Ok(None) => {
                    // No message received, continue the loop
                    continue;
                },
                Err(e) => {
                    // Error receiving message, likely channel closed
                    eprintln!("Error receiving message: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }
}
