//! AI Chat mode implementation for the interactive CLI.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;
use uuid::Uuid;
use uv_core::{PrismMultiplexer, UVLink, UVPulse};

use crate::interactive::{
    context::ExecutionContext,
    mode_handler::{ModeHandler, CommandResult},
    prompt::{render_success, render_error, render_info},
};

/// Represents a chat session with the AI
pub struct ChatSession {
    link: UVLink,
    model: String,
    max_tokens: i32,
    conversation_history: Vec<(String, String)>, // (role, content)
    context_files: Vec<(String, String)>, // (filename, content)
}

impl ChatSession {
    /// Create a new chat session
    pub fn new(
        link: UVLink,
        model: String,
        max_tokens: i32,
        context_files: Vec<(String, String)>,
    ) -> Self {
        Self {
            link,
            model,
            max_tokens,
            conversation_history: Vec::new(),
            context_files,
        }
    }
    
    /// Add a file to the context
    pub fn add_context_file(&mut self, file_path: &str) -> Result<String> {
        let path = Path::new(file_path);
        let filename = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read context file: {}", path.display()))?;
        
        self.context_files.push((filename.clone(), content));
        
        Ok(filename)
    }
    
    /// Clear the conversation history and context files
    pub fn clear(&mut self) {
        self.conversation_history.clear();
        self.context_files.clear();
    }

    /// Get the number of context files
    pub fn context_file_count(&self) -> usize {
        self.context_files.len()
    }

    /// Get context file names
    pub fn context_file_names(&self) -> Vec<&str> {
        self.context_files.iter().map(|(name, _)| name.as_str()).collect()
    }

    /// Format the conversation history into a prompt for the AI
    fn format_conversation_prompt(&self) -> String {
        let mut prompt = String::new();
        
        // Format conversation messages
        for (role, content) in &self.conversation_history {
            let role_display = if role == "user" { "User" } else { "Assistant" };
            prompt.push_str(&format!("{}: {}\n\n", role_display, content));
        }
        
        // Add file context if available
        if !self.context_files.is_empty() {
            prompt.push_str("\n--- Context Files ---\n");
            
            for (filename, content) in &self.context_files {
                prompt.push_str(&format!("\n### File: {} ###\n{}\n", filename, content));
            }
            
            prompt.push_str("\n--- End Context Files ---\n");
        }
        
        // Add final prompt for the assistant
        prompt.push_str("\n\nAssistant:");
        
        prompt
    }

    /// Send a message to the AI and process the streaming response
    pub fn send_message(&mut self, user_message: &str) -> Result<()> {
        // Add user message to conversation history
        self.conversation_history.push(("user".to_string(), user_message.to_string()));
        
        // Format the conversation history into a prompt
        let prompt = self.format_conversation_prompt();
        
        // Prepare the input for the bedrock prism
        let input = json!({
            "prompt": prompt,
            "max_tokens": self.max_tokens,
            "model": self.model
        });
        
        // Send the wavefront
        let id = Uuid::new_v4();
        self.link.send_wavefront(id, "core:bedrock", "invoke_stream", input)?;
        
        // Process the streaming response
        self.process_streaming_response(id)
    }
    
    /// Process the streaming response from the AI
    fn process_streaming_response(&mut self, _id: Uuid) -> Result<()> {
        let mut response = String::new();
        let mut in_reasoning = false;
        
        loop {
            match self.link.receive() {
                Ok(Some((_, pulse))) => {
                    match pulse {
                        UVPulse::Photon(photon) => {
                            if let Some(token) = photon.data.get("token").and_then(|t| t.as_str()) {
                                // Handle reasoning tags
                                if token == "<reasoning>\n" {
                                    in_reasoning = true;
                                    print!("{}", "Reasoning: ".bright_blue());
                                    io::stdout().flush()?;
                                } else if token == "</reasoning>\n" {
                                    in_reasoning = false;
                                    println!();
                                } else {
                                    // Print token with appropriate formatting
                                    if in_reasoning {
                                        print!("{}", token.bright_blue());
                                    } else {
                                        print!("{}", token);
                                    }
                                    io::stdout().flush()?;
                                    
                                    // Add token to response
                                    response.push_str(token);
                                }
                            }
                        },
                        UVPulse::Trap(_) => break,
                        _ => continue,
                    }
                },
                Ok(None) => continue,
                Err(e) => return Err(anyhow::format_err!("Error receiving: {}", e)),
            }
        }
        
        // Add assistant response to conversation history
        self.conversation_history.push(("assistant".to_string(), response));
        
        Ok(())
    }
}

/// Handler for AI chat mode
pub struct ChatModeHandler {
    session: Option<ChatSession>,
}

impl ChatModeHandler {
    pub fn new() -> Self {
        Self {
            session: None,
        }
    }
    
    /// Initialize chat session if not already done
    fn ensure_chat_session(&mut self, context: &ExecutionContext) -> Result<()> {
        if self.session.is_none() {
            let multiplexer = PrismMultiplexer::new();
            let link = if context.is_local() {
                multiplexer.establish_link("core:bedrock")?
            } else {
                // For remote context, we'd need to establish a remote connection
                // For now, fall back to local until remote bedrock is implemented
                render_info("Remote AI chat not yet implemented, using local Bedrock");
                multiplexer.establish_link("core:bedrock")?
            };
            
            // Default chat settings
            let model = "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string();
            let max_tokens = 4000;
            let context_files = Vec::new();
            
            self.session = Some(ChatSession::new(link, model, max_tokens, context_files));
        }
        Ok(())
    }
}

impl ModeHandler for ChatModeHandler {
    fn command(&self) -> &'static str {
        "/chat"
    }
    
    fn get_prompt_prefix(&self) -> &'static str {
        "chat"
    }
    
    fn handle_input(&mut self, input: &str, context: &mut ExecutionContext) -> Result<CommandResult> {
        // Handle special chat commands
        if input.starts_with('/') {
            match input {
                "/clear" => {
                    if let Some(session) = &mut self.session {
                        session.clear();
                        render_success("Chat history and context files cleared");
                    }
                    return Ok(CommandResult::Handled);
                },
                input if input.starts_with("/file ") => {
                    let file_path = input.trim_start_matches("/file ").trim();
                    if let Some(session) = &mut self.session {
                        match session.add_context_file(file_path) {
                            Ok(filename) => {
                                render_success(&format!("Added file to context: {}", filename));
                            },
                            Err(e) => {
                                render_error(&format!("Error adding file: {}", e));
                            }
                        }
                    }
                    return Ok(CommandResult::Handled);
                },
                "/files" => {
                    if let Some(session) = &self.session {
                        if session.context_file_count() == 0 {
                            println!("No context files loaded.");
                        } else {
                            println!("\n{} context files:", session.context_file_count());
                            for (i, filename) in session.context_file_names().iter().enumerate() {
                                println!("  {}. {}", i+1, filename);
                            }
                        }
                    } else {
                        println!("No context files loaded.");
                    }
                    return Ok(CommandResult::Handled);
                },
                _ => {
                    // Let the runtime handle other special commands
                    return Ok(CommandResult::NotHandled);
                }
            }
        }
        
        // Send message to AI
        self.ensure_chat_session(context)?;
        if let Some(session) = &mut self.session {
            print!("\n{} ", "AI:".bold().bright_green());
            io::stdout().flush()?;
            
            if let Err(e) = session.send_message(input) {
                render_error(&format!("Chat error: {}", e));
            }
            
            println!(); // Add newline after AI response
        }
        
        Ok(CommandResult::Handled)
    }
    
    fn render_help(&self) {
        println!("{}", "Chat Mode Help".bold().bright_blue());
        println!("  Send messages to the AI by typing them directly, e.g.:");
        println!("    {}{} {}", "What is Rust?".bright_green(), ":".bright_white(), "Ask the AI about Rust".bright_white());
        println!("    {}{} {}", "Explain quantum computing".bright_green(), ":".bright_white(), "Get an explanation".bright_white());
        println!("\n{}", "Chat Commands:".bold().bright_blue());
        println!("  {}{} {}", "/file <path>".bright_green(), ":".bright_white(), "Add a file to conversation context".bright_white());
        println!("  {}{} {}", "/files".bright_green(), ":".bright_white(), "List all context files".bright_white());
        println!("  {}{} {}", "/clear".bright_green(), ":".bright_white(), "Clear conversation history and context files".bright_white());
        println!("\n{}", "Special Commands:".bold().bright_blue());
        println!("  {}{} {}", "/exit".bright_green(), ":".bright_white(), "Exit chat mode".bright_white());
        println!("  {}{} {}", "/normal".bright_green(), ":".bright_white(), "Return to prism mode".bright_white());
        println!("  {}{} {}", "/help".bright_green(), ":".bright_white(), "Show this help".bright_white());
    }
    
    fn on_enter(&mut self, context: &mut ExecutionContext) -> Result<()> {
        render_success("Switched to chat mode - messages will be sent to the AI");
        render_info("Type your messages directly, '/help' for chat commands, or '/exit' to return to prism mode");
        
        // Initialize the chat session
        self.ensure_chat_session(context)?;
        
        if let Some(session) = &self.session {
            println!("Model: {}", "anthropic.claude-3-5-sonnet-20241022-v2:0".bright_green());
            if session.context_file_count() > 0 {
                println!("Context files loaded: {}", session.context_file_count());
            }
        }
        
        Ok(())
    }
    
    fn on_exit(&mut self, _context: &mut ExecutionContext) -> Result<()> {
        render_success("Exited chat mode - back to prism commands");
        Ok(())
    }
}
