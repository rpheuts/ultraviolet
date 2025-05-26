//! Command parsing for the interactive CLI.

use anyhow::{Result, anyhow};

/// Represents a parsed command from user input
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedCommand {
    /// A special command starting with '/'
    Special(SpecialCommand),
    /// A prism command with format: prism_name frequency [args...]
    Prism(PrismCommand),
    /// A shell command (when in command mode)
    Shell(String),
    /// Empty input
    Empty,
}

/// Special commands that control the CLI behavior
#[derive(Debug, Clone, PartialEq)]
pub enum SpecialCommand {
    /// Exit the CLI
    Exit,
    /// Show help
    Help,
    /// Switch to remote execution
    Remote(String), // URL
    /// Switch to local execution
    Local,
    /// List available prisms (alias for discovery list)
    Prisms,
    /// Enter command mode
    Cmd,
    /// Exit command mode (back to normal mode)
    Normal,
}

/// A prism command to be executed
#[derive(Debug, Clone, PartialEq)]
pub struct PrismCommand {
    /// Name of the prism (may include namespace)
    pub prism_name: String,
    /// Frequency to invoke
    pub frequency: String,
    /// Arguments for the command
    pub args: Vec<String>,
}

impl PrismCommand {
    /// Create a new prism command
    pub fn new(prism_name: String, frequency: String, args: Vec<String>) -> Self {
        Self {
            prism_name,
            frequency,
            args,
        }
    }
}

/// Parse user input into a command
pub fn parse_input(input: &str, mode: &crate::interactive::mode::CliMode) -> Result<ParsedCommand> {
    let input = input.trim();
    
    // Handle empty input
    if input.is_empty() {
        return Ok(ParsedCommand::Empty);
    }
    
    // Handle special commands (starting with '/')
    if input.starts_with('/') {
        return parse_special_command(input);
    }
    
    // In command mode, treat input as shell commands
    if mode.is_command() {
        return Ok(ParsedCommand::Shell(input.to_string()));
    }
    
    // Parse as prism command in normal mode
    parse_prism_command(input)
}

/// Parse a special command
fn parse_special_command(input: &str) -> Result<ParsedCommand> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    
    if parts.is_empty() {
        return Err(anyhow!("Empty special command"));
    }
    
    match parts[0] {
        "/exit" | "/quit" => Ok(ParsedCommand::Special(SpecialCommand::Exit)),
        "/help" => Ok(ParsedCommand::Special(SpecialCommand::Help)),
        "/local" => Ok(ParsedCommand::Special(SpecialCommand::Local)),
        "/prisms" => Ok(ParsedCommand::Special(SpecialCommand::Prisms)),
        "/cmd" => Ok(ParsedCommand::Special(SpecialCommand::Cmd)),
        "/normal" => Ok(ParsedCommand::Special(SpecialCommand::Normal)),
        "/remote" => {
            if parts.len() < 2 {
                return Err(anyhow!("Remote command requires a URL"));
            }
            Ok(ParsedCommand::Special(SpecialCommand::Remote(parts[1].to_string())))
        },
        _ => Err(anyhow!("Unknown special command: {}", parts[0])),
    }
}

/// Parse a prism command
fn parse_prism_command(input: &str) -> Result<ParsedCommand> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    
    if parts.len() < 2 {
        return Err(anyhow!("Prism command requires at least prism name and frequency"));
    }
    
    let prism_name = parts[0].to_string();
    let frequency = parts[1].to_string();
    let args = parts[2..].iter().map(|s| s.to_string()).collect();
    
    Ok(ParsedCommand::Prism(PrismCommand::new(prism_name, frequency, args)))
}
