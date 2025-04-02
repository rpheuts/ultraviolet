//! Typed structures for the echo prism spectrum.
//!
//! This module defines the input and output structures for the echo prism,
//! providing type safety and better developer experience.

use serde::{Deserialize, Serialize};

/// Input for the echo wavelength
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoInput {
    /// Message to echo back
    pub message: String,
}

/// Output from the echo wavelength
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoOutput {
    /// Echoed message
    pub message: String,
}
