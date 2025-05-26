//! Mode handler trait and implementations for the interactive CLI.

use anyhow::Result;
use std::io::{self, Write};
use colored::Colorize;
use serde_json::Value;
use uuid::Uuid;
use uv_core::{PrismMultiplexer, UVSpectrum, UVPulse};

use crate::interactive::{
    context::{ExecutionContext, ModeType},
    command_parser::{parse_input, ParsedCommand},
    prism_executor::execute_prism_command,
    prompt::{render_success, render_error, render_info},
};

/// Result of handling a command in a mode
#[derive(Debug)]
pub enum CommandResult {
    /// Input was successfully handled by this mode
    Handled,
    /// Input wasn't recognized by this mode handler
    NotHandled,
}

/// Trait for handling different CLI modes
pub trait ModeHandler {
    /// Get the command to enter this mode (e.g., "/cmd" or "/chat")
    #[allow(dead_code)]
    fn command(&self) -> &'static str;
    
    /// Get prompt prefix for this mode
    #[allow(dead_code)]
    fn get_prompt_prefix(&self) -> &'static str;
    
    /// Handle input in this mode
    fn handle_input(&mut self, input: &str, context: &mut ExecutionContext) -> Result<CommandResult>;
    
    /// Render help specific to this mode
    fn render_help(&self);
    
    /// Called when entering the mode
    fn on_enter(&mut self, context: &mut ExecutionContext) -> Result<()>;
    
    /// Called when exiting the mode
    fn on_exit(&mut self, context: &mut ExecutionContext) -> Result<()>;
}

/// Handler for the default prism execution mode
pub struct PrismModeHandler;

impl PrismModeHandler {
    pub fn new() -> Self {
        Self
    }
}

impl ModeHandler for PrismModeHandler {
    fn command(&self) -> &'static str {
        "/normal"
    }
    
    fn get_prompt_prefix(&self) -> &'static str {
        ""
    }
    
    fn handle_input(&mut self, input: &str, context: &mut ExecutionContext) -> Result<CommandResult> {
        // Parse input using the existing parser
        let cli_mode = match context.current_mode() {
            ModeType::Prism => crate::interactive::mode::CliMode::Normal,
            ModeType::Command => crate::interactive::mode::CliMode::Command,
            ModeType::Chat => crate::interactive::mode::CliMode::Normal,
        };
        match parse_input(input, &cli_mode) {
            Ok(ParsedCommand::Empty) => Ok(CommandResult::Handled),
            Ok(ParsedCommand::Prism(prism_cmd)) => {
                if let Err(e) = execute_prism_command(&prism_cmd, context) {
                    render_error(&format!("Execution error: {}", e));
                }
                Ok(CommandResult::Handled)
            },
            Ok(ParsedCommand::Special(_)) => {
                // Special commands should be handled by the runtime
                Ok(CommandResult::NotHandled)
            },
            Ok(ParsedCommand::Shell(_)) => {
                render_error("Shell commands not available in prism mode. Use '/cmd' to enter command mode.");
                Ok(CommandResult::Handled)
            },
            Err(e) => {
                render_error(&format!("Parse error: {}", e));
                Ok(CommandResult::Handled)
            }
        }
    }
    
    fn render_help(&self) {
        println!("{}", "Prism Mode Help".bold().bright_blue());
        println!("  Execute prism commands directly, e.g.:");
        println!("    {}{} {}", "discovery list".bright_green(), ":".bright_white(), "List available prisms".bright_white());
        println!("    {}{} {}", "bedrock invoke --model claude-3-5-sonnet \"Hello\"".bright_green(), ":".bright_white(), "Send a message to Claude".bright_white());
        println!("\n{}", "Global Commands:".bold().bright_blue());
        println!("  {}{} {}", "/cmd".bright_green(), ":".bright_white(), "Switch to command mode".bright_white());
        println!("  {}{} {}", "/chat".bright_green(), ":".bright_white(), "Switch to chat mode".bright_white());
        println!("  {}{} {}", "/help".bright_green(), ":".bright_white(), "Show this help".bright_white());
        println!("  {}{} {}", "/exit".bright_green(), ":".bright_white(), "Exit the CLI".bright_white());
    }
    
    fn on_enter(&mut self, _context: &mut ExecutionContext) -> Result<()> {
        // No special setup needed for prism mode
        Ok(())
    }
    
    fn on_exit(&mut self, _context: &mut ExecutionContext) -> Result<()> {
        // No special cleanup needed for prism mode
        Ok(())
    }
}

/// Handler for shell command execution mode
pub struct CommandModeHandler;

impl CommandModeHandler {
    pub fn new() -> Self {
        Self
    }
    
    /// Execute shell command with streaming output
    fn execute_shell_command(&self, shell_cmd: &str, context: &ExecutionContext) -> Result<()> {
        match context.location_display_name().as_str() {
            "local" => self.execute_shell_command_local(shell_cmd),
            _ => self.execute_shell_command_remote(shell_cmd, context),
        }
    }
    
    /// Execute shell command locally
    fn execute_shell_command_local(&self, shell_cmd: &str) -> Result<()> {
        let command_json = serde_json::json!({
            "shell_command": shell_cmd
        });
        
        let prism_id = "core:command";
        let spectrum = UVSpectrum::new(prism_id)?;
        let wavelength = spectrum.find_wavelength("shell_exec_stream")
            .ok_or_else(|| anyhow::anyhow!("shell_exec_stream frequency not found"))?;
        
        if let Err(e) = wavelength.input.validate(&command_json) {
            return Err(anyhow::anyhow!("Command validation failed: {}", e));
        }
        
        let multiplexer = PrismMultiplexer::new();
        let link = multiplexer.establish_link(prism_id)?;
        link.send_wavefront(Uuid::new_v4(), prism_id, "shell_exec_stream", command_json)?;
        
        loop {
            match link.receive() {
                Ok(Some((_id, pulse))) => {
                    match pulse {
                        UVPulse::Photon(photon) => {
                            if let Err(e) = self.render_command_output(&photon.data) {
                                eprintln!("Error rendering output: {}", e);
                            }
                        },
                        UVPulse::Trap(_) => break,
                        UVPulse::Extinguish => break,
                        UVPulse::Wavefront(wf) => {
                            return Err(anyhow::anyhow!("Unexpected wavefront: {}", wf.id));
                        }
                    }
                },
                Ok(None) => continue,
                Err(e) => return Err(anyhow::anyhow!("Error receiving: {}", e)),
            }
        }
        
        Ok(())
    }
    
    /// Execute shell command remotely
    fn execute_shell_command_remote(&self, _shell_cmd: &str, context: &ExecutionContext) -> Result<()> {
        // For now, just show an error - remote shell execution would need more implementation
        render_error(&format!("Remote shell execution not yet implemented for: {}", context.location_display_name()));
        Ok(())
    }
    
    /// Render command output with colors
    fn render_command_output(&self, data: &Value) -> Result<()> {
        if let Some(obj) = data.as_object() {
            let line = obj.get("line")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            let source = obj.get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("stdout");
            
            let formatted_line = if line.ends_with('\n') {
                line.to_string()
            } else {
                format!("{}\n", line)
            };
            
            match source {
                "stderr" => print!("{}", formatted_line.red()),
                "stdout" => print!("{}", formatted_line),
                "system" => print!("{}", formatted_line.dimmed()),
                _ => print!("{}", formatted_line),
            }
            
            io::stdout().flush()?;
        } else {
            println!("{}", data);
            io::stdout().flush()?;
        }
        
        Ok(())
    }
}

impl ModeHandler for CommandModeHandler {
    fn command(&self) -> &'static str {
        "/cmd"
    }
    
    fn get_prompt_prefix(&self) -> &'static str {
        "cmd"
    }
    
    fn handle_input(&mut self, input: &str, context: &mut ExecutionContext) -> Result<CommandResult> {
        // Handle special commands that start with /
        if input.starts_with('/') {
            // Let the runtime handle special commands
            return Ok(CommandResult::NotHandled);
        }
        
        // Execute as shell command
        if let Err(e) = self.execute_shell_command(input, context) {
            render_error(&format!("Shell execution error: {}", e));
        }
        
        Ok(CommandResult::Handled)
    }
    
    fn render_help(&self) {
        println!("{}", "Command Mode Help".bold().bright_blue());
        println!("  Execute shell commands directly, e.g.:");
        println!("    {}{} {}", "ls -la".bright_green(), ":".bright_white(), "List files with details".bright_white());
        println!("    {}{} {}", "ps aux".bright_green(), ":".bright_white(), "Show running processes".bright_white());
        println!("    {}{} {}", "echo \"Hello World\"".bright_green(), ":".bright_white(), "Echo text with quotes".bright_white());
        println!("\n{}", "Special Commands:".bold().bright_blue());
        println!("  {}{} {}", "/exit".bright_green(), ":".bright_white(), "Exit command mode".bright_white());
        println!("  {}{} {}", "/normal".bright_green(), ":".bright_white(), "Return to prism mode".bright_white());
        println!("  {}{} {}", "/help".bright_green(), ":".bright_white(), "Show this help".bright_white());
    }
    
    fn on_enter(&mut self, _context: &mut ExecutionContext) -> Result<()> {
        render_success("Switched to command mode - shell commands will be executed directly");
        render_info("Type '/exit' to return to prism mode, or '/help' for command help");
        Ok(())
    }
    
    fn on_exit(&mut self, _context: &mut ExecutionContext) -> Result<()> {
        render_success("Exited command mode - back to prism commands");
        Ok(())
    }
}

/// Handler for AI chat mode (placeholder for now)
pub struct ChatModeHandler;

impl ChatModeHandler {
    pub fn new() -> Self {
        Self
    }
}

impl ModeHandler for ChatModeHandler {
    fn command(&self) -> &'static str {
        "/chat"
    }
    
    fn get_prompt_prefix(&self) -> &'static str {
        "chat"
    }
    
    fn handle_input(&mut self, input: &str, _context: &mut ExecutionContext) -> Result<CommandResult> {
        // Handle special chat commands
        if input.starts_with('/') {
            match input {
                "/clear" => {
                    render_success("Chat history cleared (placeholder)");
                    return Ok(CommandResult::Handled);
                },
                _ => {
                    // Let the runtime handle other special commands
                    return Ok(CommandResult::NotHandled);
                }
            }
        }
        
        // Placeholder for chat functionality
        render_info(&format!("Chat mode placeholder - you said: {}", input));
        render_info("Chat functionality will be implemented in a future update");
        
        Ok(CommandResult::Handled)
    }
    
    fn render_help(&self) {
        println!("{}", "Chat Mode Help".bold().bright_blue());
        println!("  Chat mode is currently a placeholder and will be implemented soon.");
        println!("\n{}", "Special Commands:".bold().bright_blue());
        println!("  {}{} {}", "/exit".bright_green(), ":".bright_white(), "Exit chat mode".bright_white());
        println!("  {}{} {}", "/normal".bright_green(), ":".bright_white(), "Return to prism mode".bright_white());
        println!("  {}{} {}", "/help".bright_green(), ":".bright_white(), "Show this help".bright_white());
    }
    
    fn on_enter(&mut self, _context: &mut ExecutionContext) -> Result<()> {
        render_success("Switched to chat mode (placeholder)");
        render_info("Chat functionality will be implemented in a future update");
        render_info("Type '/exit' to return to prism mode, or '/help' for help");
        Ok(())
    }
    
    fn on_exit(&mut self, _context: &mut ExecutionContext) -> Result<()> {
        render_success("Exited chat mode - back to prism commands");
        Ok(())
    }
}
