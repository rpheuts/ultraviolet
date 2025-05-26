use anyhow::Result;
use colored::Colorize;
use serde_json::Value;
use uuid::Uuid;
use std::io::{self, Write};
use uv_core::{PrismMultiplexer, UVPulse, UVSpectrum};

use crate::interactive::{context::ExecutionContext, mode_handler::{CommandResult, ModeHandler}, prompt::{render_error, render_info, render_success}};

pub struct ShellModeHandler;

impl ShellModeHandler {
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

impl ModeHandler for ShellModeHandler {
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
