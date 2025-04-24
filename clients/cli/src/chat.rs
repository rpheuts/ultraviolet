use std::fs;
use std::io::{self, Write};
use std::path::Path;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;
use uuid::Uuid;
use uv_core::{PrismMultiplexer, UVLink, UVPulse};

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

/// Load context files from paths
fn load_context_files(file_paths: Vec<String>) -> Result<Vec<(String, String)>> {
    let mut context_files = Vec::new();
    
    for path in file_paths {
        let path = Path::new(&path);
        let filename = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read context file: {}", path.display()))?;
        
        context_files.push((filename, content));
    }
    
    Ok(context_files)
}

/// Handle the chat command
pub fn handle_chat(model: &str, max_tokens: i32, context_files: Vec<String>) -> Result<()> {
    println!("{}", "Starting AI Chat Session".bold().bright_purple());
    println!("Type your messages and press Enter. Type '/exit' or '/quit' to end the session.");
    println!("Type {} to see available commands.", "/help".bright_green());
    println!("Model: {}", model.bright_green());
    
    // Load context files
    let loaded_context_files = load_context_files(context_files)?;
    if !loaded_context_files.is_empty() {
        println!("Loaded {} context files:", loaded_context_files.len());
        for (filename, _) in &loaded_context_files {
            println!("  - {}", filename);
        }
    }
    
    // Create chat session
    let multiplexer = PrismMultiplexer::new();
    let link = multiplexer.establish_link("core:bedrock")?;
    let mut session = ChatSession::new(link, model.to_string(), max_tokens, loaded_context_files);
    
    // Chat loop
    loop {
        print!("\n{} ", "You:".bold().bright_yellow());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        // Check for exit/quit commands
        if input == "/exit" || input == "/quit" {
            break;
        }
        
        // Handle clear command
        if input == "/clear" {
            session.clear();
            println!("{} Conversation history and context files cleared.", "✓".bright_green());
            continue;
        }
        
        // Check for special commands
        if input.starts_with("/file ") {
            let file_path = input.trim_start_matches("/file ").trim();
            match session.add_context_file(file_path) {
                Ok(filename) => {
                    println!("{} Added file to context: {}", "✓".bright_green(), filename);
                },
                Err(e) => {
                    println!("{} Error adding file: {}", "✗".bright_red(), e);
                }
            }
            continue;
        }
        
        // Handle files command to list context files
        if input == "/files" {
            if session.context_files.is_empty() {
                println!("No context files loaded.");
            } else {
                println!("\n{} context files:", session.context_files.len());
                for (i, (filename, _)) in session.context_files.iter().enumerate() {
                    println!("  {}. {}", i+1, filename);
                }
            }
            continue;
        }
        
        // Handle help command
        if input == "/help" {
            println!("\n{}", "Available commands:".bold().bright_blue());
            println!("  {}{} {}", "/file <path>".bright_green(), ":".bright_white(), "Add a file to the conversation context".bright_white());
            println!("  {}{} {}", "/files".bright_green(), ":".bright_white(), "List all context files".bright_white());
            println!("  {}{} {}", "/clear".bright_green(), ":".bright_white(), "Clear conversation history and context files".bright_white());
            println!("  {}{} {}", "/help".bright_green(), ":".bright_white(), "Show this help message".bright_white());
            println!("  {}{} {}", "/exit".bright_green(), ":".bright_white(), "Exit the chat session".bright_white());
            println!("  {}{} {}", "/quit".bright_green(), ":".bright_white(), "Exit the chat session".bright_white());
            continue;
        }
        
        print!("\n{} ", "AI:".bold().bright_green());
        io::stdout().flush()?;
        
        session.send_message(input)?;
        
        println!(); // Add a newline after the AI response
    }
    
    println!("Chat session ended.");
    Ok(())
}
