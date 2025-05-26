//! Mode handler trait and implementations for the interactive CLI.

use anyhow::Result;
use crate::interactive::{context::{ExecutionContext}};

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