//! Pulse protocol types for the Ultraviolet system.
//!
//! The pulse protocol consists of three fundamental components:
//! - Wavefront: Initial request with frequency and input data
//! - Photon: Response data carrier
//! - Trap: Completion signal with optional error information

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::UVError;

/// The main enum representing all message types in the pulse protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UVPulse {
    /// Initial request that starts a pulse
    Wavefront(Wavefront),
    
    /// Response data carrier
    Photon(Photon),
    
    /// Completion/error signal
    Trap(Trap),
    
    /// Signal to terminate a prism and its refractions
    Extinguish,
}

/// Initial request that starts a pulse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wavefront {
    /// Unique identifier for correlation
    pub id: Uuid,
    
    /// The frequency (method) to invoke
    pub frequency: String,
    
    /// Input data structured according to the frequency's input schema
    pub input: Value,
}

/// Response data carrier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photon {
    /// Correlation ID matching the wavefront
    pub id: Uuid,
    
    /// Data structured according to the frequency's output schema
    pub data: Value,
}

/// Completion signal with optional error information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trap {
    /// Correlation ID matching the wavefront
    pub id: Uuid,
    
    /// Optional error (None indicates successful completion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<UVError>,
}
