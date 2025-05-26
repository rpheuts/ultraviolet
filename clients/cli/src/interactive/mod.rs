//! Interactive CLI module for the Ultraviolet client.
//!
//! This module provides an interactive REPL interface for executing
//! UV prism commands either locally or remotely.

mod cli;
mod context;
mod command_parser;
mod prism_executor;
mod prompt;
mod modes;

pub use cli::{handle_interactive, handle_interactive_with_mode};
pub use context::ModeType;
