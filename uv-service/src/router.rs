//! Pulse router for the UV Service.
//!
//! This module handles routing pulse messages to the appropriate prism handlers.

use std::sync::Arc;

use uv_core::PrismMultiplexer;

/// Router for pulse messages.
///
/// The PulseRouter is responsible for determining which prism should handle
/// a particular message, and for managing connections to those prisms.
pub struct PulseRouter {
    /// The multiplexer for establishing prism connections
    _multiplexer: Arc<PrismMultiplexer>,
}

impl PulseRouter {
    /// Create a new PulseRouter with the given multiplexer.
    pub fn new(multiplexer: Arc<PrismMultiplexer>) -> Self {
        Self { _multiplexer: multiplexer }
    }
    
    /// Parse a prism and frequency from a given string.
    ///
    /// The expected format is "namespace:name:frequency" or "namespace:name" with
    /// the frequency provided separately.
    pub fn parse_prism_and_frequency(&self, fullpath: &str) -> (String, String) {
        let parts: Vec<&str> = fullpath.split(':').collect();
        
        if parts.len() >= 3 {
            // Format is namespace:name:frequency
            let prism = format!("{}:{}", parts[0], parts[1]);
            let frequency = parts[2..].join(":");
            
            (prism, frequency)
        } else if parts.len() == 2 {
            // Format is namespace:name with frequency provided separately
            (fullpath.to_string(), String::new())
        } else {
            // Invalid format, use default
            ("example:echo".to_string(), fullpath.to_string())
        }
    }
}
