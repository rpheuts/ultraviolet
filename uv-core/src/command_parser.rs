//! Command parsing utilities for Ultraviolet.
//!
//! This module provides functionality for parsing command line arguments into
//! structured commands that can be executed by prisms. It handles the parsing
//! of prism IDs, frequencies, and arguments according to the schema defined in
//! a prism's spectrum.

use serde_json::Value;
use thiserror::Error;
use crate::spectrum::UVSpectrum;
use crate::args_processor::ArgsProcessor;
use crate::UVError;

/// Errors that can occur during command parsing.
#[derive(Debug, Error)]
pub enum CommandParserError {
    #[error("Missing prism ID")]
    MissingPrismId,
    
    #[error("Invalid prism ID: {0}")]
    InvalidPrismId(String),
    
    #[error("Failed to load spectrum: {0}")]
    SpectrumError(#[from] UVError),
    
    #[error("Error processing arguments: {0}")]
    ArgumentError(String),
    
    #[error("Unknown frequency: {0}")]
    UnknownFrequency(String),
}

/// A type alias for the command parser result.
pub type Result<T> = std::result::Result<T, CommandParserError>;

/// A parsed command request ready for execution.
#[derive(Debug, Clone)]
pub struct CommandRequest {
    /// Full prism ID (namespace:name format)
    pub prism_id: String,
    
    /// Frequency to call (method name)
    pub frequency: String,
    
    /// Structured input based on the schema
    pub structured_input: Value,
}

/// Parse a raw command into a structured command request.
///
/// # Arguments
///
/// * `args` - Vector of command line arguments, where:
///   - args[0] should be the prism ID (namespace:name)
///   - args[1] should be the frequency (defaults to "help" if missing)
///   - args[2..] are the arguments for the frequency
///
/// # Returns
///
/// A `CommandRequest` containing the parsed and validated command.
pub fn parse_command(args: &[String]) -> Result<CommandRequest> {
    // 1. Extract prism_id
    let prism_id = args.get(0)
        .ok_or(CommandParserError::MissingPrismId)?
        .clone();
    
    // Validate prism_id format (namespace:name)
    if !prism_id.contains(':') {
        return Err(CommandParserError::InvalidPrismId(format!(
            "Prism ID '{}' does not follow the namespace:name format", prism_id
        )));
    }
    
    // 2. Extract frequency (default to "help")
    let frequency = args.get(1)
        .cloned()
        .unwrap_or_else(|| "help".to_string());
    
    // 3. Load the spectrum
    let spectrum = UVSpectrum::new(&prism_id)
        .map_err(CommandParserError::SpectrumError)?;
    
    // 4. Check if frequency exists
    if spectrum.find_wavelength(&frequency).is_none() {
        return Err(CommandParserError::UnknownFrequency(frequency));
    }
    
    // 5. Process the arguments using the schema
    let processor = ArgsProcessor::from_spectrum(&spectrum, &frequency)
        .map_err(|e| CommandParserError::ArgumentError(e.to_string()))?;
    
    // Skip the first two args (prism_id and frequency)
    let command_args = if args.len() > 2 { &args[2..] } else { &[] };
    
    let structured_input = processor.process_args(command_args)
        .map_err(|e| CommandParserError::ArgumentError(e.to_string()))?;
    
    Ok(CommandRequest {
        prism_id,
        frequency,
        structured_input,
    })
}

/// Extract the raw command arguments from a full command line.
///
/// This function filters out global CLI arguments that are meant for the
/// client itself, not for the prism.
///
/// # Arguments
///
/// * `args` - Full set of command line arguments
/// * `global_args` - List of global argument prefixes to filter out
///
/// # Returns
///
/// A vector of arguments that comprise the actual prism command.
pub fn extract_command_args(args: &[String], global_args: &[&str]) -> Vec<String> {
    let mut result = Vec::new();
    let mut skip_next = false;
    
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        
        // Check if this is a global argument
        let is_global = global_args.iter().any(|prefix| arg.starts_with(prefix));
        
        if is_global {
            // Assume the next argument might be a value for this flag
            skip_next = true;
        } else {
            result.push(arg.clone());
        }
    }
    
    result
}
