//! Parser for command line arguments.
//!
//! This module provides utilities for parsing command line arguments into
//! structured representations that can be processed against schemas.

use std::collections::HashMap;

/// Result of parsing command line arguments
#[derive(Debug, Clone)]
pub struct ParsedArgs {
    /// Named arguments (--name value or --name=value)
    pub named_args: HashMap<String, Vec<String>>,
    
    /// Positional arguments (not prefixed with - or --)
    pub positional_args: Vec<String>,
    
    /// Boolean flags (--flag)
    pub flags: HashMap<String, bool>,
}

impl ParsedArgs {
    /// Create a new empty ParsedArgs
    pub fn new() -> Self {
        Self {
            named_args: HashMap::new(),
            positional_args: Vec::new(),
            flags: HashMap::new(),
        }
    }
    
    /// Add a named argument
    pub fn add_named_arg(&mut self, name: &str, value: &str) {
        self.named_args.entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(value.to_string());
    }
    
    /// Add a boolean flag
    pub fn add_flag(&mut self, name: &str) {
        self.flags.insert(name.to_string(), true);
    }
    
    /// Add a positional argument
    pub fn add_positional_arg(&mut self, value: &str) {
        self.positional_args.push(value.to_string());
    }
}

/// Parse a list of command line arguments
///
/// This handles common patterns like:
/// - Named args: `--name value` or `--name=value`
/// - Boolean flags: `--flag`
/// - Positional args: anything not starting with `-` or `--`
///
/// # Arguments
/// * `args` - List of command line arguments to parse
///
/// # Returns
/// * `ParsedArgs` - Structured representation of the arguments
pub fn parse_args(args: &[String]) -> ParsedArgs {
    let mut result = ParsedArgs::new();
    let mut i = 0;
    
    while i < args.len() {
        let arg = &args[i];
        
        if arg.starts_with("--") {
            let name = &arg[2..]; // strip --
            
            // Check for --name=value format
            if let Some(eq_pos) = name.find('=') {
                let (key, value) = name.split_at(eq_pos);
                // Skip the = in value
                result.add_named_arg(key, &value[1..]);
            }
            // Check for --name value format
            else if i + 1 < args.len() && !args[i+1].starts_with('-') {
                result.add_named_arg(name, &args[i+1]);
                i += 1; // Skip the next arg as we've used it as a value
            }
            // Must be a flag (--flag)
            else {
                result.add_flag(name);
            }
        }
        // Short options like -f (future expansion)
        else if arg.starts_with('-') && arg.len() > 1 {
            // For now, just treat like a positional arg
            result.add_positional_arg(arg);
        }
        // Positional argument
        else {
            result.add_positional_arg(arg);
        }
        
        i += 1;
    }
    
    result
}
