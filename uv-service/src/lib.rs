//! UV Service implementation for the Ultraviolet system.
//!
//! This crate provides a WebSocket server that enables clients to interact with prisms
//! using the same Pulse Protocol used internally by prisms. The service acts as a
//! bridge between WebSocket clients and the prism ecosystem.

pub mod server;
pub mod router;
pub mod options;

use std::net::SocketAddr;
use std::path::PathBuf;

use thiserror::Error;
use uv_core::UVError;

/// Error type for UV Service operations
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
    
    #[error("Prism error: {0}")]
    PrismError(#[from] UVError),
    
    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Server error: {0}")]
    ServerError(String),
}

/// Type alias for service results
pub type Result<T> = std::result::Result<T, ServiceError>;

/// Configuration options for the UV Service.
#[derive(Debug, Clone)]
pub struct ServiceOptions {
    /// Address to bind the server to
    pub bind_address: SocketAddr,
    
    /// Whether to enable TLS
    pub enable_tls: bool,
    
    /// Path to TLS certificate file (if TLS enabled)
    pub cert_path: Option<PathBuf>,
    
    /// Path to TLS key file (if TLS enabled)
    pub key_path: Option<PathBuf>,
    
    /// Whether to serve static files
    pub serve_static: bool,
    
    /// Path to static files directory (if serving static files)
    pub static_dir: Option<PathBuf>,
}

impl Default for ServiceOptions {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:3000".parse().unwrap(),
            enable_tls: false,
            cert_path: None,
            key_path: None,
            serve_static: false,
            static_dir: None,
        }
    }
}

/// Start the UV Service with the given options.
///
/// This is the main entry point for starting the service programmatically.
pub async fn start_service(options: ServiceOptions) -> Result<()> {
    server::run_server(options).await
}
