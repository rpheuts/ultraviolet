//! Core interactive CLI implementation using the new mode-aware architecture.

use anyhow::Result;
use rustyline::{Editor, error::ReadlineError, Config};
use std::collections::HashMap;
use colored::Colorize;

use crate::interactive::{
    context::{ExecutionContext, ModeType},
    mode_handler::{ModeHandler, CommandResult, PrismModeHandler, CommandModeHandler},
    chat_mode::ChatModeHandler,
    prompt::{render_welcome, render_error},
};

/// Main interactive CLI session
pub struct InteractiveCli {
    /// Execution context (manages location and mode)
    context: ExecutionContext,
    /// Rustyline editor for input handling
    editor: Editor<(), rustyline::history::DefaultHistory>,
    /// Registered mode handlers
    mode_handlers: HashMap<ModeType, Box<dyn ModeHandler>>,
}

impl InteractiveCli {
    /// Create a new interactive CLI session
    pub fn new() -> Result<Self> {
        // Configure rustyline with better error handling and history management
        let config = Config::builder()
            .history_ignore_space(true)
            .history_ignore_dups(true)?
            .completion_type(rustyline::CompletionType::List)
            .build();
        
        let editor = Editor::with_config(config)?;
        
        let mut cli = Self {
            context: ExecutionContext::new(),
            editor,
            mode_handlers: HashMap::new(),
        };
        
        // Register built-in mode handlers
        cli.register_mode_handler(ModeType::Prism, Box::new(PrismModeHandler::new()));
        cli.register_mode_handler(ModeType::Command, Box::new(CommandModeHandler::new()));
        cli.register_mode_handler(ModeType::Chat, Box::new(ChatModeHandler::new()));
        
        Ok(cli)
    }
    
    /// Register a mode handler
    fn register_mode_handler(&mut self, mode_type: ModeType, handler: Box<dyn ModeHandler>) {
        self.mode_handlers.insert(mode_type, handler);
    }
    
    /// Run the interactive CLI loop
    pub fn run(&mut self) -> Result<()> {
        render_welcome();
        
        loop {
            // Get current mode and its handler
            let mode = self.context.current_mode().clone();
            let input = match self.get_user_input() {
                Ok(input) => input,
                Err(e) => {
                    render_error(&format!("Input error: {}", e));
                    continue;
                }
            };
            
            // Check for global mode switch commands first
            if let Some(new_mode) = self.check_mode_switch(&input) {
                // Handle mode switching
                match new_mode {
                    Some(target_mode) => {
                        // Exit current mode
                        if let Some(handler) = self.mode_handlers.get_mut(&mode) {
                            let _ = handler.on_exit(&mut self.context);
                        }
                        
                        // Enter new mode
                        self.context.enter_mode(target_mode.clone());
                        if let Some(handler) = self.mode_handlers.get_mut(&target_mode) {
                            let _ = handler.on_enter(&mut self.context);
                        }
                    },
                    None => {
                        // Handle /exit command
                        if !self.context.exit_mode() {
                            break; // Exit CLI
                        } else {
                            // Exited a mode, call on_exit for old mode and on_enter for new mode
                            let new_mode = self.context.current_mode().clone();
                            if let Some(handler) = self.mode_handlers.get_mut(&new_mode) {
                                let _ = handler.on_enter(&mut self.context);
                            }
                        }
                    }
                }
                continue;
            }
            
            // Handle /help command globally
            if input == "/help" {
                self.render_global_help();
                continue;
            }
            
            // Let the current mode handler process the input
            if let Some(handler) = self.mode_handlers.get_mut(&mode) {
                match handler.handle_input(&input, &mut self.context)? {
                    CommandResult::Handled => continue,
                    CommandResult::NotHandled => {
                        render_error(&format!("Unknown command: {}", input));
                        render_error("Type '/help' for available commands");
                    },
                }
            } else {
                render_error("Internal error: no handler for current mode");
                break;
            }
        }
        
        Ok(())
    }
    
    /// Get user input using rustyline with mode-aware prompt
    fn get_user_input(&mut self) -> Result<String> {
        let prompt = format!("[{}]> ", self.context.full_display_name());
        
        match self.editor.readline(&prompt) {
            Ok(line) => {
                // Add non-empty lines to rustyline's history
                if !line.trim().is_empty() {
                    let _ = self.editor.add_history_entry(&line);
                }
                Ok(line.trim().to_string())
            },
            Err(ReadlineError::Interrupted) => {
                // User pressed Ctrl+C
                Ok("/exit".to_string())
            },
            Err(ReadlineError::Eof) => {
                // User pressed Ctrl+D
                Ok("/exit".to_string())
            },
            Err(err) => {
                Err(anyhow::anyhow!("Readline error: {}", err))
            }
        }
    }
    
    /// Check if input is a mode switch command or exit command
    /// Returns Some(Some(mode)) for mode switch, Some(None) for exit, None for not a global command
    fn check_mode_switch(&self, input: &str) -> Option<Option<ModeType>> {
        match input {
            "/cmd" => Some(Some(ModeType::Command)),
            "/chat" => Some(Some(ModeType::Chat)),
            "/normal" => Some(Some(ModeType::Prism)),
            "/exit" => Some(None), // Signal exit
            _ => None, // Not a global command
        }
    }
    
    /// Render global help information
    fn render_global_help(&self) {
        println!("{}", "Ultraviolet Interactive CLI".bold().bright_purple());
        println!();
        
        // Show current mode info
        let current_mode = self.context.current_mode();
        if let Some(handler) = self.mode_handlers.get(current_mode) {
            println!("{} ({})", "Current Mode:".bold().bright_blue(), current_mode_name(current_mode));
            handler.render_help();
            println!();
        }
        
        // Show global commands
        println!("{}", "Global Commands:".bold().bright_blue());
        println!("  {}{} {}", "/cmd".bright_green(), ":".bright_white(), "Switch to command mode (execute shell commands)".bright_white());
        println!("  {}{} {}", "/chat".bright_green(), ":".bright_white(), "Switch to chat mode (AI conversation)".bright_white());
        println!("  {}{} {}", "/normal".bright_green(), ":".bright_white(), "Switch to normal mode (prism commands)".bright_white());
        println!("  {}{} {}", "/help".bright_green(), ":".bright_white(), "Show this help message".bright_white());
        println!("  {}{} {}", "/exit".bright_green(), ":".bright_white(), "Exit current mode or CLI".bright_white());
        
        println!("\n{}", "Remote Connection:".bold().bright_blue());
        println!("  {}{} {}", "/remote <url>".bright_green(), ":".bright_white(), "Connect to remote UV server".bright_white());
        println!("  {}{} {}", "/local".bright_green(), ":".bright_white(), "Switch to local execution".bright_white());
    }
}

/// Get the display name for a mode
fn current_mode_name(mode: &ModeType) -> &'static str {
    match mode {
        ModeType::Prism => "Prism Mode",
        ModeType::Command => "Command Mode", 
        ModeType::Chat => "Chat Mode",
    }
}

/// Handle the interactive CLI command - main entry point
pub fn handle_interactive() -> Result<()> {
    handle_interactive_with_mode(None)
}

/// Handle the interactive CLI command with an optional initial mode
pub fn handle_interactive_with_mode(initial_mode: Option<ModeType>) -> Result<()> {
    // Check if we're in a proper terminal environment
    if !atty::is(atty::Stream::Stdout) || !atty::is(atty::Stream::Stdin) {
        eprintln!("Error: Interactive CLI requires a proper terminal environment");
        eprintln!("Make sure you're running this command in a terminal, not through a pipe or redirect");
        return Err(anyhow::anyhow!("Not running in a terminal"));
    }
    
    let mut cli = InteractiveCli::new()?;
    
    // If an initial mode is specified, switch to it immediately
    if let Some(mode) = initial_mode {
        // Exit the default mode (Prism)
        let current_mode = cli.context.current_mode().clone();
        if let Some(handler) = cli.mode_handlers.get_mut(&current_mode) {
            let _ = handler.on_exit(&mut cli.context);
        }
        
        // Enter the new mode
        cli.context.enter_mode(mode.clone());
        if let Some(handler) = cli.mode_handlers.get_mut(&mode) {
            let _ = handler.on_enter(&mut cli.context);
        }
    }
    
    cli.run()
}
