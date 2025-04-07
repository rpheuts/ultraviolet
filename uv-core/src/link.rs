//! Link interface for the Ultraviolet system.
//!
//! The UVLink provides a bidirectional communication channel between system components,
//! with high-level methods for sending and receiving pulse components.

use std::sync::Arc;
use std::time::Duration;
use serde::de::DeserializeOwned;
use serde_json::Value;
use uuid::Uuid;

use crate::pulse::{UVPulse, Wavefront, Photon, Trap};
use crate::error::{UVError, Result};
use crate::transport::{Transport, create_transport_pair};

/// Bidirectional communication channel between system components.
#[derive(Clone)]
pub struct UVLink {
    transport: Arc<dyn Transport>,
    // Other fields may be added later
}

impl Drop for UVLink {
    fn drop(&mut self) {
        // Send an Extinguish pulse before dropping the link
        // We ignore errors here since we're shutting down anyway
        let _ = self.send_extinguish();
        
        // Close the transport
        let _ = self.transport.close();
    }
}

impl UVLink {
    /// Create a pair of connected links.
    ///
    /// This function creates a pair of links that are connected to each other.
    /// Data sent on one link can be received on the other.
    pub fn create_link() -> (UVLink, UVLink) {
        // Create a pair of transports
        let (transport1, transport2) = create_transport_pair();
        
        // Create links with the transports
        (
            UVLink { transport: Arc::from(transport1) },
            UVLink { transport: Arc::from(transport2) }
        )
    }
    
    /// Send a wavefront to initiate a request.
    ///
    /// This function creates a wavefront with the given ID, prism, frequency, and input data,
    /// and sends it over the transport.
    pub fn send_wavefront(&self, id: Uuid, prism: &str, frequency: &str, input: Value) -> Result<()> {
        let wavefront = Wavefront {
            id,
            prism: prism.to_string(),
            frequency: frequency.to_string(),
            input,
        };
        
        self.send_pulse(UVPulse::Wavefront(wavefront))
    }
    
    /// Send an extinguish signal to terminate a prism.
    ///
    /// This function sends an Extinguish pulse over the transport.
    pub fn send_extinguish(&self) -> Result<()> {
        self.send_pulse(UVPulse::Extinguish)
    }
    
    /// Emit a photon as a response.
    ///
    /// This function creates a photon with the given ID and data,
    /// and sends it over the transport.
    pub fn emit_photon(&self, id: Uuid, data: Value) -> Result<()> {
        let photon = Photon {
            id,
            data,
        };
        
        self.send_pulse(UVPulse::Photon(photon))
    }
    
    /// Emit a trap to signal completion or error.
    ///
    /// This function creates a trap with the given ID and optional error,
    /// and sends it over the transport.
    pub fn emit_trap(&self, id: Uuid, error: Option<UVError>) -> Result<()> {
        let trap = Trap {
            id,
            error,
        };
        
        self.send_pulse(UVPulse::Trap(trap))
    }
    
    /// Send a raw pulse.
    ///
    /// This function sends the pulse over the transport.
    pub fn send_pulse(&self, pulse: UVPulse) -> Result<()> {
        self.transport.send(pulse)
    }
    
    /// Receive the next pulse component.
    ///
    /// This function receives a pulse from the transport with a timeout,
    /// and returns the pulse along with its ID.
    pub fn receive(&self) -> Result<Option<(Uuid, UVPulse)>> {
        // Receive from the transport
        if let Some(pulse) = self.transport.receive()? {
            // Extract the ID from the pulse
            let id = match &pulse {
                UVPulse::Wavefront(wavefront) => wavefront.id,
                UVPulse::Photon(photon) => photon.id,
                UVPulse::Trap(trap) => trap.id,
                UVPulse::Extinguish => Uuid::nil(), // No ID for Extinguish
            };
            
            Ok(Some((id, pulse)))
        } else {
            Ok(None)
        }
    }
    
    /// Absorb all photons and deserialize into the expected type.
    ///
    /// This function receives pulses until it gets a trap, collecting all photon data
    /// along the way. It then deserializes the collected data into the expected type.
    pub fn absorb<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut data = Vec::new();
        
        // Collect all photons until we get a trap
        loop {
            match self.receive()? {
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
                    std::thread::sleep(Duration::from_millis(10));
                },
            }
        }
        
        // If we have multiple photons, combine them into an array
        let result = if data.len() > 1 {
            serde_json::to_value(data)?
        } else if data.len() == 1 {
            data.into_iter().next().unwrap()
        } else {
            // No data received
            return Err(UVError::Other("No data received".to_string()));
        };
        
        // Deserialize into the expected type
        let typed_result = serde_json::from_value(result)?;
        Ok(typed_result)
    }
}
