//! Configuration options for the UV Service.
//!
//! This module defines the command-line options and configuration settings
//! for the UV Service.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

use crate::{ServiceOptions, LogLevel};

/// Command-line arguments for the UV Service.
#[derive(Parser, Debug)]
#[command(author, version, about = "WebSocket service for the Ultraviolet system")]
pub struct CliOptions {
    /// Address to bind the server to
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    pub bind: String,
    
    /// Enable TLS for secure WebSocket connections
    #[arg(short, long)]
    pub tls: bool,
    
    /// Path to TLS certificate file (required if TLS is enabled)
    #[arg(long)]
    pub cert: Option<PathBuf>,
    
    /// Path to TLS key file (required if TLS is enabled)
    #[arg(long)]
    pub key: Option<PathBuf>,
    
    /// Serve static files from the specified directory
    #[arg(short, long)]
    pub static_dir: Option<PathBuf>,
}

impl CliOptions {
    /// Convert CLI options to ServiceOptions
    pub fn into_service_options(self) -> Result<ServiceOptions, String> {
        // Parse bind address
        let bind_address = self.bind.parse::<SocketAddr>()
            .map_err(|e| format!("Invalid bind address: {}", e))?;
        
        // Check TLS options
        if self.tls {
            if self.cert.is_none() {
                return Err("TLS certificate file must be specified with --cert when TLS is enabled".to_string());
            }
            
            if self.key.is_none() {
                return Err("TLS key file must be specified with --key when TLS is enabled".to_string());
            }
        }
        
        Ok(ServiceOptions {
            bind_address,
            enable_tls: self.tls,
            cert_path: self.cert,
            key_path: self.key,
            serve_static: self.static_dir.is_some(),
            static_dir: self.static_dir,
            init_tracing: true,
            log_level: LogLevel::Normal,
        })
    }
}
