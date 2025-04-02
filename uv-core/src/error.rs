//! Error types for the Ultraviolet system.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Main error type for the Ultraviolet system.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum UVError {
    /// A requested frequency (method) was not found
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Input validation failed
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Error during execution of a frequency
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Error in the transport layer
    #[error("Transport error: {0}")]
    TransportError(String),

    /// Error resolving a refraction
    #[error("Refraction error: {0}")]
    RefractionError(String),

    /// Error in property mapping
    #[error("Property mapping error: {0}")]
    PropertyMappingError(String),

    /// Serialization or deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Generic error with custom message
    #[error("{0}")]
    Other(String),
}

/// Result type alias for Ultraviolet operations
pub type Result<T> = std::result::Result<T, UVError>;

impl From<serde_json::Error> for UVError {
    fn from(err: serde_json::Error) -> Self {
        UVError::SerializationError(err.to_string())
    }
}

impl From<std::io::Error> for UVError {
    fn from(err: std::io::Error) -> Self {
        UVError::Other(format!("IO error: {}", err))
    }
}

impl From<libloading::Error> for UVError {
    fn from(err: libloading::Error) -> Self {
        UVError::Other(format!("Library loading error: {}", err))
    }
}
