//! UV Service command-line entry point.
//!
//! This module provides the main entry point for running the UV Service
//! as a standalone process.

use clap::Parser;

use uv_service::{
    options::CliOptions,
    start_service,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let cli_opts = CliOptions::parse();
    
    // Convert to service options
    let service_opts = cli_opts
        .into_service_options()
        .map_err(|e| format!("Failed to parse options: {}", e))?;
    
    // Start the service
    start_service(service_opts).await?;
    
    Ok(())
}
