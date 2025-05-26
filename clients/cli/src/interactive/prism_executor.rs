//! Prism execution logic for the interactive CLI.

use anyhow::Result;
use crate::interactive::{
    context::ExecutionContext,
    command_parser::PrismCommand,
    prompt::render_error,
};
use crate::{local, remote};

/// Execute a prism command based on the current execution context
pub fn execute_prism_command(
    command: &PrismCommand,
    context: &ExecutionContext,
) -> Result<()> {
    match context.is_local() {
        true => {
            execute_local_prism_command(command)
        },
        false => {
            if let Some(url) = context.remote_url() {
                execute_remote_prism_command(command, url)
            } else {
                Err(anyhow::anyhow!("Remote context but no URL available"))
            }
        }
    }
}

/// Execute a prism command locally
fn execute_local_prism_command(command: &PrismCommand) -> Result<()> {
    // Build arguments in the format expected by handle_local
    let mut args = vec![command.frequency.clone()];
    args.extend(command.args.clone());
    
    // Call the existing local handler
    match local::handle_local(&command.prism_name, args, None) {
        Ok(()) => Ok(()),
        Err(e) => {
            render_error(&format!("Local execution failed: {}", e));
            Err(e)
        }
    }
}

/// Execute a prism command on a remote server
fn execute_remote_prism_command(command: &PrismCommand, remote_url: &str) -> Result<()> {
    // Build arguments in the format expected by handle_remote
    let mut args = vec![command.frequency.clone()];
    args.extend(command.args.clone());
    
    // Call the existing remote handler
    match remote::handle_remote(remote_url, &command.prism_name, args, None) {
        Ok(()) => Ok(()),
        Err(e) => {
            render_error(&format!("Remote execution failed: {}", e));
            Err(e)
        }
    }
}
