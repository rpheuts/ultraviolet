//! CLI mode definitions for the interactive CLI.

/// Represents the current mode of the interactive CLI
#[derive(Debug, Clone, PartialEq)]
pub enum CliMode {
    /// Normal prism execution mode
    Normal,
    /// Shell command execution mode
    Command,
}

impl CliMode {
    /// Check if the mode is command
    pub fn is_command(&self) -> bool {
        matches!(self, Self::Command)
    }
}

impl Default for CliMode {
    fn default() -> Self {
        Self::Normal
    }
}
