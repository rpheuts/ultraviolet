//! Prompt rendering for the interactive CLI.

use colored::Colorize;
use crate::interactive::{context::{ExecutionContext, ModeType}};

use super::modes::mode::CliMode;

/// Render the interactive prompt based on the current context and mode
#[allow(dead_code)]
pub fn render_prompt(context: &ExecutionContext, mode: &CliMode) -> String {
    let context_name = if context.is_local() {
        "local".bright_blue()
    } else {
        context.full_display_name().bright_green()
    };
    
    let mode_name = match mode {
        CliMode::Normal => "normal".bright_white(),
        CliMode::Command => "cmd".bright_yellow(),
    };
    
    format!("[{}|{}]> ", context_name, mode_name)
}

/// Render a welcome message when entering interactive mode
pub fn render_welcome() {
    println!("{}", "Welcome to Ultraviolet Interactive CLI".bold().bright_purple());
    println!("Type prism commands in the format: {} {} {}", 
        "prism_name".bright_cyan(), 
        "frequency".bright_yellow(), 
        "[args...]".bright_white()
    );
    println!("Special commands start with '{}' - type {} for help.", 
        "/".bright_magenta(), 
        "/help".bright_green()
    );
    println!();
}

/// Render an error message
pub fn render_error(message: &str) {
    println!("{} {}", "Error:".bright_red().bold(), message);
}

/// Render a success message
pub fn render_success(message: &str) {
    println!("{} {}", "✓".bright_green(), message);
}

/// Render an info message
pub fn render_info(message: &str) {
    println!("{} {}", "ℹ".bright_blue(), message);
}

/// Render a fancy Starship-style two-line prompt
pub fn render_fancy_prompt(context: &ExecutionContext) -> String {
    let location_icon = if context.is_local() {
        "🌐"
    } else {
        "🌍"
    };
    
    let location_name = if context.is_local() {
        "local".bright_blue()
    } else {
        context.location_display_name().bright_green()
    };
    
    let (mode_icon, mode_name, mode_color) = match context.current_mode() {
        ModeType::Prism => ("🔮", "prism", "bright_magenta"),
        ModeType::Command => ("💻", "shell", "bright_yellow"),
        ModeType::Chat => ("🤖", "chat", "bright_green"),
    };
    
    let mode_colored = match mode_color {
        "bright_magenta" => format!("{} {}", mode_icon, mode_name).bright_magenta(),
        "bright_yellow" => format!("{} {}", mode_icon, mode_name).bright_yellow(),
        "bright_green" => format!("{} {}", mode_icon, mode_name).bright_green(),
        _ => format!("{} {}", mode_icon, mode_name).bright_white(),
    };
    
    format!(
        "╭{} [{} {}] as [{}]  \n╰❯ ",
        "uv".bold().bright_purple(),
        location_icon,
        location_name,
        mode_colored
    )
}
