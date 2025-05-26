//! Prompt rendering for the interactive CLI.

use colored::Colorize;
use crate::interactive::{context::ExecutionContext, mode::CliMode};

/// Render the interactive prompt based on the current context and mode
#[allow(dead_code)]
pub fn render_prompt(context: &ExecutionContext, mode: &CliMode) -> String {
    let context_name = if context.is_local() {
        "local".bright_blue()
    } else {
        context.full_display_name().bright_green()
    };
    
    let mode_name = match mode {
        CliMode::Normal => "normal".bright_white(),
        CliMode::Command => "cmd".bright_yellow(),
    };
    
    format!("[{}|{}]> ", context_name, mode_name)
}

/// Render a welcome message when entering interactive mode
pub fn render_welcome() {
    println!("{}", "Welcome to Ultraviolet Interactive CLI".bold().bright_purple());
    println!("Type prism commands in the format: {} {} {}", 
        "prism_name".bright_cyan(), 
        "frequency".bright_yellow(), 
        "[args...]".bright_white()
    );
    println!("Special commands start with '{}' - type {} for help.", 
        "/".bright_magenta(), 
        "/help".bright_green()
    );
    println!();
}

/// Render help information for the interactive CLI
pub fn render_help() {
    println!("\n{}", "Interactive CLI Help".bold().bright_blue());
    println!();
    
    println!("{}", "Special Commands:".bold());
    println!("  {}{} {}", "/help".bright_green(), ":".bright_white(), "Show this help message".bright_white());
    println!("  {}{} {}", "/prisms".bright_green(), ":".bright_white(), "List available prisms".bright_white());
    println!("  {}{} {}", "/cmd".bright_green(), ":".bright_white(), "Enter command mode (execute shell commands)".bright_white());
    println!("  {}{} {}", "/normal".bright_green(), ":".bright_white(), "Return to normal mode (prism commands)".bright_white());
    println!("  {}{} {}", "/local".bright_green(), ":".bright_white(), "Switch to local execution mode".bright_white());
    println!("  {}{} {}", "/remote <url>".bright_green(), ":".bright_white(), "Switch to remote execution mode".bright_white());
    println!("  {}{} {}", "/exit".bright_green(), ":".bright_white(), "Exit the interactive CLI".bright_white());
    println!("  {}{} {}", "/quit".bright_green(), ":".bright_white(), "Exit the interactive CLI".bright_white());
    println!();
    
    println!("{}", "Prism Commands:".bold());
    println!("  Execute prism commands using: {} {} {}", 
        "prism_name".bright_cyan(), 
        "frequency".bright_yellow(), 
        "[args...]".bright_white()
    );
    println!("  Examples:");
    println!("    {}", "bedrock invoke --prompt \"Hello world\"".bright_white());
    println!("    {}", "aws:accounts list".bright_white());
    println!("    {}", "core:command exec ls -la".bright_white());
    println!();
    
    println!("{}", "Command Mode:".bold());
    println!("  Use {} to enter command mode for direct shell execution", "/cmd".bright_green());
    println!("  In command mode, type shell commands directly:");
    println!("    {}", "ls -la".bright_white());
    println!("    {}", "ps aux | grep rust".bright_white());
    println!("    {}", "git status".bright_white());
    println!("  Use {} to return to normal prism mode", "/normal".bright_green());
    println!();
    
    println!("{}", "Context:".bold());
    println!("  Commands are executed in the current context (local or remote)");
    println!("  Use {} to switch between local and remote execution", "/local".bright_green());
    println!("  Use {} to connect to a remote server", "/remote <url>".bright_green());
    println!();
    println!("{}", "Remote URL Examples:".bold());
    println!("  {} → {}", "192.168.1.100".bright_cyan(), "ws://192.168.1.100:4000/ws".bright_white());
    println!("  {} → {}", "192.168.1.100:3000".bright_cyan(), "ws://192.168.1.100:3000/ws".bright_white());
    println!("  {} → {}", "http://server.com".bright_cyan(), "ws://server.com/ws".bright_white());
    println!("  {} → {}", "ws://server.com:4000/ws".bright_cyan(), "ws://server.com:4000/ws".bright_white());
    println!();
}

/// Render an error message
pub fn render_error(message: &str) {
    println!("{} {}", "Error:".bright_red().bold(), message);
}

/// Render a success message
pub fn render_success(message: &str) {
    println!("{} {}", "✓".bright_green(), message);
}

/// Render an info message
pub fn render_info(message: &str) {
    println!("{} {}", "ℹ".bright_blue(), message);
}
