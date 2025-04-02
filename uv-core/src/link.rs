//! Link interface for the Ultraviolet system.
//!
//! The UVLink provides a bidirectional communication channel between system components,
//! with high-level methods for sending and receiving pulse components.

use std::sync::Arc;
use serde::{Serialize, de::DeserializeOwned};
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
    
    /// Establish a link with a prism.
    ///
    /// This function calls the prism's on_link_established method to set up
    /// the communication channel.
    pub async fn establish_with(&self, prism: &mut dyn crate::prism::UVPrism) -> Result<()> {
        prism.link_established(self).await
    }
    
    /// Send a wavefront to initiate a request.
    ///
    /// This function creates a wavefront with the given ID, frequency, and input data,
    /// and sends it over the transport.
    pub async fn send_wavefront(&self, id: Uuid, frequency: &str, input: Value) -> Result<()> {
        let wavefront = Wavefront {
            id,
            frequency: frequency.to_string(),
            input,
        };
        
        self.send_pulse(UVPulse::Wavefront(wavefront)).await
    }
    
    /// Emit a photon as a response.
    ///
    /// This function creates a photon with the given ID and data,
    /// and sends it over the transport.
    pub async fn emit_photon(&self, id: Uuid, data: Value) -> Result<()> {
        let photon = Photon {
            id,
            data,
        };
        
        self.send_pulse(UVPulse::Photon(photon)).await
    }
    
    /// Emit a trap to signal completion or error.
    ///
    /// This function creates a trap with the given ID and optional error,
    /// and sends it over the transport.
    pub async fn emit_trap(&self, id: Uuid, error: Option<UVError>) -> Result<()> {
        let trap = Trap {
            id,
            error,
        };
        
        self.send_pulse(UVPulse::Trap(trap)).await
    }
    
    /// Send a raw pulse.
    ///
    /// This function serializes the pulse and sends it over the transport.
    pub async fn send_pulse(&self, pulse: UVPulse) -> Result<()> {
        // Serialize the pulse to JSON
        let json = serde_json::to_string(&pulse)?;
        
        // Send the JSON over the transport
        self.transport.send(json.into_bytes()).await
    }
    
    /// Receive the next pulse component.
    ///
    /// This function receives raw data from the transport, deserializes it into a pulse,
    /// and returns the pulse along with its ID.
    pub async fn receive(&self) -> Result<Option<(Uuid, UVPulse)>> {
        // Receive raw data from the transport
        if let Some(data) = self.transport.receive().await? {
            // Deserialize the data into a pulse
            let pulse: UVPulse = serde_json::from_slice(&data)?;
            
            // Extract the ID from the pulse
            let id = match &pulse {
                UVPulse::Wavefront(wavefront) => wavefront.id,
                UVPulse::Photon(photon) => photon.id,
                UVPulse::Trap(trap) => trap.id,
                _ => Uuid::nil(), // For pulses without an ID
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
    pub async fn absorb<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut data = Vec::new();
        
        // Collect all photons until we get a trap
        while let Some((_, pulse)) = self.receive().await? {
            match pulse {
                UVPulse::Photon(photon) => {
                    data.push(photon.data);
                },
                UVPulse::Trap(trap) => {
                    // If there's an error, return it
                    if let Some(error) = trap.error {
                        return Err(error);
                    }
                    // Otherwise, we're done collecting
                    break;
                },
                _ => continue, // Ignore other pulse types
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
    
    /// Reflect data back as photon(s) and a success trap.
    ///
    /// This function serializes the data, sends it as one or more photons,
    /// and then sends a success trap.
    pub async fn reflect<T>(&self, id: Uuid, data: T) -> Result<()>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(data)?;
        
        // If it's an array, send multiple photons
        if let Value::Array(items) = value {
            for item in items {
                self.emit_photon(id, item).await?;
            }
        } else {
            // Otherwise, send a single photon
            self.emit_photon(id, value).await?;
        }
        
        // Send a success trap
        self.emit_trap(id, None).await?;
        Ok(())
    }
}
