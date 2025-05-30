//! UV Service implementation for the Ultraviolet system.
//!
//! This crate provides a WebSocket server that enables clients to interact with prisms
//! using the same Pulse Protocol used internally by prisms. The service acts as a
//! bridge between WebSocket clients and the prism ecosystem.

pub mod server;
pub mod request_handler;
pub mod uv_handler;

use std::net::SocketAddr;
use std::path::PathBuf;

use server::UVServer;
use thiserror::Error;
use uv_core::UVError;

/// Get the default assets directory path (~/.uv/assets)
pub fn get_default_assets_dir() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_default();
    path.push(".uv");
    path.push("assets");
    path
}

/// Get the web assets directory path (~/.uv/assets/web)
pub fn get_web_assets_dir() -> PathBuf {
    let mut path = get_default_assets_dir();
    path.push("web");
    path
}

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

/// Log level for the service
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    /// Show detailed debug logs
    Debug,
    
    /// Show only warnings and errors (default)
    Normal,
    
    /// Show only critical errors
    Quiet,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Normal
    }
}

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
    
    /// Whether to initialize the tracing system
    /// Set to false when used as an embedded service to avoid conflicts
    pub init_tracing: bool,
    
    /// Log level setting
    pub log_level: LogLevel,
}

impl Default for ServiceOptions {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:3000".parse().unwrap(),
            enable_tls: false,
            cert_path: None,
            key_path: None,
            serve_static: true,  // Enable by default
            static_dir: Some(get_web_assets_dir()),  // Use default web assets dir
            init_tracing: true,
            log_level: LogLevel::Normal,
        }
    }
}

/// Start the UV Service with the given options.
///
/// This is the main entry point for starting the service programmatically.
pub async fn start_service(options: ServiceOptions) -> Result<()> {
    UVServer::run_server(options).await
}
