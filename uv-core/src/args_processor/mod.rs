//! Command argument processing functionality.
//!
//! This module provides utilities for parsing and processing command line arguments
//! based on a JSON Schema definition. It handles complex argument patterns like
//! named arguments, flags, positional arguments, and more.

mod processor;
mod parser;

pub use processor::ArgsProcessor;
pub use processor::ProcessorError;
