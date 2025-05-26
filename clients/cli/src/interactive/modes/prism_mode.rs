use anyhow::Result;
use colored::Colorize;

use crate::interactive::{command_parser::{parse_input, ParsedCommand}, context::ExecutionContext, mode_handler::{CommandResult, ModeHandler}, prism_executor::execute_prism_command, prompt::render_error, ModeType};

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