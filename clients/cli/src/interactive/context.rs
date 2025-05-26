//! Execution context management for the interactive CLI.

use anyhow::Result;
use std::fmt;

/// Represents the current execution location for the interactive CLI
#[derive(Debug, Clone)]
pub enum ExecutionLocation {
    /// Commands executed locally
    Local,
    /// Commands executed on a remote server
    #[allow(dead_code)]
    Remote(String), // URL of the remote server
}

/// Represents the available CLI modes
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ModeType {
    /// Normal prism execution mode (base mode)
    Prism,
    /// Shell command execution mode
    Command,
    /// AI chat mode
    Chat,
}

/// Represents the current execution context for the interactive CLI
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Current execution location (local or remote)
    location: ExecutionLocation,
    /// Mode stack for nested mode support (bottom is base mode)
    mode_stack: Vec<ModeType>,
}

impl ExecutionContext {
    /// Create a new execution context with default values
    pub fn new() -> Self {
        Self {
            location: ExecutionLocation::Local,
            mode_stack: vec![ModeType::Prism], // Start in prism mode
        }
    }
    
    /// Create a new local execution context
    #[allow(dead_code)]
    pub fn local() -> Self {
        Self::new()
    }
    
    /// Create a new remote execution context
    #[allow(dead_code)]
    pub fn remote(url: String) -> Self {
        Self {
            location: ExecutionLocation::Remote(url),
            mode_stack: vec![ModeType::Prism],
        }
    }
    
    /// Check if the context is local
    pub fn is_local(&self) -> bool {
        matches!(self.location, ExecutionLocation::Local)
    }
    
    /// Check if the context is remote
    #[allow(dead_code)]
    pub fn is_remote(&self) -> bool {
        matches!(self.location, ExecutionLocation::Remote(_))
    }
    
    /// Get the remote URL if this is a remote context
    #[allow(dead_code)]
    pub fn remote_url(&self) -> Option<&str> {
        match &self.location {
            ExecutionLocation::Remote(url) => Some(url),
            ExecutionLocation::Local => None,
        }
    }
    
    /// Get a display name for the execution location (for prompts)
    pub fn location_display_name(&self) -> String {
        match &self.location {
            ExecutionLocation::Local => "local".to_string(),
            ExecutionLocation::Remote(url) => {
                // Extract hostname from URL for cleaner display
                if let Ok(parsed) = url::Url::parse(url) {
                    if let Some(host) = parsed.host_str() {
                        return host.to_string();
                    }
                }
                // Fallback to full URL if parsing fails
                url.clone()
            }
        }
    }
    
    /// Switch to a remote context
    #[allow(dead_code)]
    pub fn switch_to_remote(&mut self, url: String) -> Result<()> {
        // TODO: Validate URL format
        self.location = ExecutionLocation::Remote(url);
        Ok(())
    }
    
    /// Switch to local context
    #[allow(dead_code)]
    pub fn switch_to_local(&mut self) {
        self.location = ExecutionLocation::Local;
    }
    
    /// Enter a new mode, pushing it onto the mode stack
    pub fn enter_mode(&mut self, mode: ModeType) {
        self.mode_stack.push(mode);
    }
    
    /// Exit the current mode, returning to the previous one
    /// Returns true if successfully exited a mode, false if at base mode
    pub fn exit_mode(&mut self) -> bool {
        if self.mode_stack.len() > 1 {
            self.mode_stack.pop();
            true
        } else {
            false // At base mode, cannot exit further
        }
    }
    
    /// Get the current active mode
    pub fn current_mode(&self) -> &ModeType {
        self.mode_stack.last().unwrap_or(&ModeType::Prism)
    }
    
    /// Check if currently in a specific mode
    #[allow(dead_code)]
    pub fn is_in_mode(&self, mode: &ModeType) -> bool {
        self.current_mode() == mode
    }
    
    /// Get the mode stack depth (useful for nested mode indicators)
    #[allow(dead_code)]
    pub fn mode_depth(&self) -> usize {
        self.mode_stack.len()
    }
    
    /// Get a display name that includes both location and mode
    pub fn full_display_name(&self) -> String {
        let location = self.location_display_name();
        match self.current_mode() {
            ModeType::Prism => location,
            ModeType::Command => format!("{}|cmd", location),
            ModeType::Chat => format!("{}|chat", location),
        }
    }
}

impl fmt::Display for ExecutionContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_display_name())
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}
