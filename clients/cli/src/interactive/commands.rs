//! Special command handlers for the interactive CLI.
//! This file is kept for future expansion but most functionality has moved to mode handlers.

use anyhow::Result;
use crate::interactive::{
    context::ExecutionContext,
    command_parser::{SpecialCommand, PrismCommand},
    prism_executor::execute_prism_command,
    prompt::{render_help, render_success, render_error, render_info},
    mode::CliMode,
};

// NOTE: Most command handling has been moved to the new mode handler architecture.
// These functions are kept for potential future use or reference.

/// Handle a special command and return whether to continue the CLI loop
/// NOTE: This function is deprecated in favor of the new mode handler architecture
#[allow(dead_code)]
pub fn handle_special_command(
    command: &SpecialCommand,
    context: &mut ExecutionContext,
    mode: &mut CliMode,
) -> Result<bool> {
    match command {
        SpecialCommand::Exit => {
            render_info("Exiting UV CLI");
            Ok(false) // Exit the loop
        },
        SpecialCommand::Help => {
            render_help();
            Ok(true) // Continue the loop
        },
        SpecialCommand::Local => {
            if context.is_local() {
                render_info("Already in local mode");
            } else {
                context.switch_to_local();
                render_success("Switched to local execution mode");
            }
            Ok(true) // Continue the loop
        },
        SpecialCommand::Remote(url) => {
            // Format the URL to ensure it's a proper WebSocket URL
            match format_remote_url(url) {
                Ok(formatted_url) => {
                    // Validate the formatted URL
                    if let Err(e) = validate_remote_url(&formatted_url) {
                        render_error(&format!("Invalid URL format: {}", e));
                        return Ok(true);
                    }
                
                    // Switch to remote context with the formatted URL
                    match context.switch_to_remote(formatted_url) {
                        Ok(()) => {
                            render_success(&format!("Connected to remote server: {}", context.full_display_name()));
                        },
                        Err(e) => {
                            render_error(&format!("Failed to connect to remote server: {}", e));
                        }
                    }
                },
                Err(e) => {
                    render_error(&format!("Failed to format URL: {}", e));
                }
            }
            Ok(true) // Continue the loop
        },
        SpecialCommand::Prisms => {
            // Execute "discovery list" command
            if let Err(e) = execute_discovery_list(context) {
                render_error(&format!("Failed to list prisms: {}", e));
            }
            Ok(true) // Continue the loop
        },
        SpecialCommand::Cmd => {
            if mode.is_command() {
                render_info("Already in command mode");
            } else {
                *mode = CliMode::Command;
                render_success("Switched to command mode - shell commands will be executed directly");
                render_info("Type '/normal' to return to prism mode, or '/exit' to quit");
            }
            Ok(true) // Continue the loop
        },
        
        SpecialCommand::Normal => {
            if mode.is_normal() {
                render_info("Already in normal mode");
            } else {
                *mode = CliMode::Normal;
                render_success("Switched to normal mode - back to prism commands");
            }
            Ok(true) // Continue the loop
        },
    }
}

/// Format user input into a proper WebSocket URL
/// NOTE: This function is deprecated but kept for potential future use
#[allow(dead_code)]
pub fn format_remote_url(input: &str) -> Result<String> {
    let input = input.trim();
    
    // If it's already a WebSocket URL, return as-is
    if input.starts_with("ws://") || input.starts_with("wss://") {
        return Ok(input.to_string());
    }
    
    // If it has http/https, convert to ws/wss and add /ws if missing
    if input.starts_with("http://") || input.starts_with("https://") {
        let ws_scheme = if input.starts_with("https://") { "wss" } else { "ws" };
        let without_scheme = input.split_once("://").unwrap().1;
        
        let formatted = if without_scheme.contains("/ws") {
            format!("{}://{}", ws_scheme, without_scheme)
        } else {
            format!("{}://{}/ws", ws_scheme, without_scheme.trim_end_matches('/'))
        };
        
        return Ok(formatted);
    }
    
    // If it contains a colon but no protocol, assume it's host:port
    if input.contains(':') && !input.contains("://") {
        return Ok(format!("ws://{}/ws", input));
    }
    
    // If it's just a hostname/IP, add default port 4000
    if !input.contains(':') && !input.contains("://") {
        return Ok(format!("ws://{}:4000/ws", input));
    }
    
    // Fallback: assume it's a hostname and add the full format
    Ok(format!("ws://{}:4000/ws", input))
}

/// Validate a remote URL (basic validation)
/// NOTE: This function is deprecated but kept for potential future use
#[allow(dead_code)]
pub fn validate_remote_url(url: &str) -> Result<()> {
    // Basic URL validation
    let parsed = url::Url::parse(url)?;
    
    // Ensure it has a supported scheme
    match parsed.scheme() {
        "ws" | "wss" => Ok(()),
        scheme => Err(anyhow::anyhow!("Unsupported URL scheme: {}. Use ws:// or wss://", scheme)),
    }
}

/// Execute the discovery list command (alias for /prisms)
/// NOTE: This function is deprecated but kept for potential future use
#[allow(dead_code)]
fn execute_discovery_list(context: &ExecutionContext) -> Result<()> {
    let discovery_command = PrismCommand::new(
        "discovery".to_string(),
        "list".to_string(),
        vec![]
    );
    
    execute_prism_command(&discovery_command, context)
}
