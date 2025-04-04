//! Ultraviolet Command Line Interface.
//!
//! This crate provides a command line interface for interacting with Ultraviolet prisms.
//! It can either run in client mode, executing commands against prisms (local or remote),
//! or in service mode, running a WebSocket server that clients can connect to.

use std::io::stdout;

use anyhow::Result;
use clap::Parser;
use tracing::{ error, debug };
use uv_service::{start_service, ServiceOptions};
use uv_ui::UIInferenceEngine;

mod args;
mod renderer;
mod ws_client;

use args::Cli;
use renderer::CliRenderer;
use ws_client::{execute_with_embedded, execute_remote};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize tracing with filter based on debug/quiet flags
    let filter = if cli.debug {
        "uv_cli=debug,uv_service=debug"
    } else if cli.quiet {
        "uv_cli=error,uv_service=error"
    } else {
        "uv_cli=warn,uv_service=warn"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    // Check if we're in service mode
    if cli.service {
        // Run as a WebSocket server
        println!("Starting UV service on {}", cli.bind);
        
        // Create service options
        let bind_address = cli.bind.parse()?;
        let log_level = if cli.debug {
            uv_service::LogLevel::Debug
        } else if cli.quiet {
            uv_service::LogLevel::Quiet
        } else {
            uv_service::LogLevel::Normal
        };

        let options = ServiceOptions {
            bind_address,
            enable_tls: cli.tls,
            cert_path: cli.cert,
            key_path: cli.key,
            serve_static: cli.static_dir.is_some(),
            static_dir: cli.static_dir,
            init_tracing: false, // Don't initialize tracing again (already done in CLI)
            log_level,
        };
        
        // Run the server until terminated
        start_service(options).await.map_err(|e| anyhow::anyhow!("Service error: {}", e))?;
        
        return Ok(());
    }
    
    // We're in client mode
    let prism = cli.prism.as_ref().expect("Prism is required when not in service mode");
    let binding = "help".to_string();
    let frequency = cli.frequency.as_ref().unwrap_or(&binding);
    
    debug!("Executing {} for prism {}", frequency, prism);
    
    // Filter global CLI arguments to get just the command args
    let global_arg_prefixes = [
        "--service", "--remote", "--secure", "--bind", "--tls", 
        "--cert", "--key", "--static-dir", "--raw", "--no-color",
        "--debug", "--quiet"
    ];
    
    // Extract command-specific arguments using the command parser helper
    let filtered_args = uv_core::command_parser::extract_command_args(
        &cli.args, 
        &global_arg_prefixes
    );
    
    debug!("Executing {} for prism {} with args: {:?}", frequency, prism, filtered_args);
    
    // Set up direct debug logging if debug flag is set
    if cli.debug {
        println!("DEBUG ENABLED: About to execute command");
    }
    
    // Get the raw schema if it starts with @
    let mut filtered_args = filtered_args.clone(); 
    for i in 0..filtered_args.len() {
        if filtered_args[i].starts_with("--schema=@") {
            if let Some(path) = filtered_args[i].strip_prefix("--schema=@") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    filtered_args[i] = format!("--schema={}", content);
                    if cli.debug {
                        println!("DEBUG: Replaced file reference with content: {}", content);
                    }
                }
            }
        }
    }
    
    // Execute the command
    let result = if let Some(remote) = &cli.remote {
        // Connect to a remote service
        debug!("Using remote service at {}", remote);
        execute_remote(remote, cli.secure, prism, frequency, &filtered_args).await?
    } else {
        // Start embedded service and execute locally
        debug!("Using embedded service");
        execute_with_embedded(prism, frequency, &filtered_args).await?
    };
    
    // Debug logging
    if cli.debug {
        println!("DEBUG: Command result: {}", serde_json::to_string_pretty(&result).unwrap_or_default());
    }
    
    if cli.raw {
        // Output raw JSON
        match serde_json::to_string_pretty(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error serializing result: {}", e),
        }
    } else {
        // Use UI inference and rendering
        let engine = UIInferenceEngine::new();
        match engine.infer(&result) {
            Ok(component) => {
                // Create a renderer with color setting
                let renderer = CliRenderer::new().with_color(!cli.no_color);
                if let Err(e) = renderer.render(&component, &mut stdout()) {
                    error!("Error rendering result: {}", e);
                    
                    // Fall back to JSON
                    match serde_json::to_string_pretty(&result) {
                        Ok(json) => println!("{}", json),
                        Err(e) => eprintln!("Error serializing result: {}", e),
                    }
                }
            },
            Err(e) => {
                // Fall back to JSON on inference error
                error!("UI inference failed, falling back to JSON: {}", e);
                match serde_json::to_string_pretty(&result) {
                    Ok(json) => println!("{}", json),
                    Err(e) => eprintln!("Error serializing result: {}", e),
                }
            }
        }
    }
    
    Ok(())
}
