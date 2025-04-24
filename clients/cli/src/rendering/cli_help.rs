use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::{collections::HashMap, fs, path::PathBuf};
use uv_core::UVSpectrum;

/// Render help information for the chat command
pub fn render_chat_help() -> Result<()> {
    println!("{}", "Chat Command".bold().bright_purple());
    println!("{}", "============".bright_purple());
    println!();
    println!("{}", "Start an interactive AI chat session using Bedrock LLMs".bright_white());
    println!();
    
    println!("{}", "Options:".underline().bright_blue());
    println!("  {}{} {}", "-m, --model".bright_green(), ":".bright_white(), "Specify the LLM model to use".bright_white());
    println!("      {}: {}", "Default".bright_white(), "us.anthropic.claude-3-7-sonnet-20250219-v1:0".bright_cyan());
    println!();
    println!("  {}{} {}", "--max-tokens".bright_green(), ":".bright_white(), "Maximum number of tokens for the response".bright_white());
    println!("      {}: {}", "Default".bright_white(), "4096".bright_cyan());
    println!();
    println!("  {}{} {}", "-c, --context".bright_green(), ":".bright_white(), "Add a file as context for the AI (can be used multiple times)".bright_white());
    println!();
    
    println!("{}", "Available Models:".underline().bright_blue());
    println!("  {}: {}", "Claude 3.7".bright_green(), "us.anthropic.claude-3-7-sonnet-20250219-v1:0".bright_white());
    println!("  {}: {}", "Claude 3.5".bright_green(), "us.anthropic.claude-3-5-sonnet-20241022-v2:0".bright_white());
    println!("  {}: {}", "DeepSeek R1".bright_green(), "us.deepseek.r1-v1:0".bright_white());
    println!("  {}: {}", "Llama 3".bright_green(), "us.meta.llama3-1-405b-instruct-v1:0".bright_white());
    println!("  {}: {}", "AWS Nova".bright_green(), "us.amazon.nova-pro-v1:0".bright_white());
    println!();
    
    println!("{}", "Usage examples:".underline().bright_blue());
    println!("  {}", "uv chat".bright_green());
    println!("  {}", "uv chat --model us.anthropic.claude-3-5-sonnet-20241022-v2:0".bright_green());
    println!("  {}", "uv chat --context file1.txt --context file2.py".bright_green());
    println!();
    
    println!("{}", "Chat Commands:".underline().bright_blue());
    println!("  {}: {}", "exit".bright_yellow(), "Exit the chat session".bright_white());
    println!("  {}: {}", "quit".bright_yellow(), "Exit the chat session".bright_white());
    
    Ok(())
}

/// Render help information for the server command
pub fn render_server_help() -> Result<()> {
    println!("{}", "Server Command".bold().bright_purple());
    println!("{}", "==============".bright_purple());
    println!();
    println!("{}", "Run Ultra-Violet in server mode".bright_white());
    println!();
    
    println!("{}", "Options:".underline().bright_blue());
    println!("  {}{} {}", "-a, --address".bright_green(), ":".bright_white(), "Local address and port to host UV on".bright_white());
    println!("      {}: {}", "Default".bright_white(), "127.0.0.1:3000".bright_cyan());
    println!();
    println!("  {}{} {}", "--no_static".bright_green(), ":".bright_white(), "Don't host the static files".bright_white());
    println!("      {}: {}", "Default".bright_white(), "false (static files are hosted)".bright_cyan());
    println!();
    
    println!("{}", "Usage examples:".underline().bright_blue());
    println!("  {}", "uv server".bright_green());
    println!("  {}", "uv server --address 0.0.0.0:8080".bright_green());
    println!("  {}", "uv server --no_static".bright_green());
    
    Ok(())
}

/// Handle help request based on provided prism and args
pub fn handle_help_request(prism: &str, args: &[String]) -> Result<()> {
    // Determine the help level based on the inputs
    if prism == "help" || prism == "--help" || prism == "-h" {
        // Case: Global help (should be handled before reaching here, but as a fallback)
        return render_global_help();
    }
    
    // Check for built-in commands
    if prism == "chat" {
        return render_chat_help();
    }
    
    if prism == "server" {
        return render_server_help();
    }
    
    // Try to load the prism spectrum
    let (name, namespace) = match UVSpectrum::resolve_prism_id(&prism.to_string()) {
        Ok(result) => result,
        Err(_) => {
            // If prism can't be resolved but help was requested, show global help
            return render_global_help();
        }
    };
    
    let prism_id = format!("{}:{}", namespace, name);
    let spectrum = match UVSpectrum::new(&prism_id) {
        Ok(s) => s,
        Err(_) => {
            // If spectrum can't be loaded but help was requested, at least show global help
            return render_global_help();
        }
    };
    
    // If the args contain a potential wavelength name, show wavelength help
    if args.len() >= 1 && args[0] != "--help" && args[0] != "-h" {
        let frequency = &args[0];
        
        // Check if the wavelength exists
        if let Some(_wavelength) = spectrum.find_wavelength(frequency) {
            return render_frequency_help(&spectrum, frequency);
        }
    }
    
    // Default to prism-level help
    render_prism_help(&spectrum)
}

/// Render global help information showing all available prisms
pub fn render_global_help() -> Result<()> {
    let prisms = scan_available_prisms()?;
    
    println!("{}", "UV - Ultraviolet CLI".bold().bright_purple());
    println!("{}", "====================".bright_purple());
    println!();
    println!("{}", "Available prisms:".underline().bright_blue());
    
    for (namespace, prism_list) in group_prisms_by_namespace(&prisms) {
        println!("  {}:", namespace.bold());
        for prism in prism_list {
            println!("    {}{} {}", 
                     prism.name.bright_green(), 
                     ":".bright_white(),
                     prism.description.bright_white());
        }
    }
    
    println!();
    println!("{}", "Built-in commands:".underline().bright_blue());
    println!("  {}{} {}", "server".bright_green(), ":".bright_white(), "Run Ultra-Violet in server mode".bright_white());
    println!("  {}{} {}", "chat".bright_green(), ":".bright_white(), "Start an interactive AI chat session using Bedrock LLMs".bright_white());
    
    println!();
    println!("{}", "Usage:".underline().bright_blue());
    println!("  {} {} {}", "uv".bright_green(), "server".bright_yellow(), "[options]".bright_white());
    println!("  {} {} {}", "uv".bright_green(), "chat".bright_yellow(), "[options]".bright_white());
    println!("  {} {} {}", "uv".bright_green(), "[prism]".bright_yellow(), "[frequency] [options]".bright_white());
    println!("  {} {} {}    {}", "uv".bright_green(), "[prism]".bright_yellow(), "--help".bright_cyan(), "Show help for a prism".bright_white());
    println!("  {} {} {} {}    {}", "uv".bright_green(), "[prism]".bright_yellow(), "[frequency]".bright_yellow(), "--help".bright_cyan(), "Show help for a specific frequency".bright_white());
    
    Ok(())
}

/// Render help information for a specific prism
pub fn render_prism_help(spectrum: &UVSpectrum) -> Result<()> {
    println!("{} {} - {}", 
             spectrum.name.bold().bright_green(), 
             "Prism".bold(),
             spectrum.description.bright_white());
    println!("{}", "=".repeat(spectrum.name.len() + spectrum.description.len() + 9).bright_purple());
    println!();
    
    if !spectrum.tags.is_empty() {
        println!("Tags: {}", spectrum.tags.join(", ").bright_cyan());
        println!();
    }
    
    println!("{}", "Available frequencies:".underline().bright_blue());
    for wavelength in &spectrum.wavelengths {
        println!("  {}{} {}", 
                 wavelength.frequency.bright_green(), 
                 ":".bright_white(),
                 wavelength.description.bright_white());
    }
    
    if !spectrum.refractions.is_empty() {
        println!();
        println!("{}", "Refractions (dependencies):".underline().bright_blue());
        for refraction in &spectrum.refractions {
            println!("  {}{} → {}", 
                     refraction.name.bright_yellow(), 
                     ":".bright_white(),
                     refraction.target.bright_white());
        }
    }
    
    println!();
    println!("{}", "Usage:".underline().bright_blue());
    println!("  {} {}:{} {}",
             "uv".bright_green(),
             spectrum.namespace.bright_yellow(),
             spectrum.name.bright_yellow(),
             "[frequency] [options]".bright_white());
    println!("  {} {}:{} {} {}",
             "uv".bright_green(),
             spectrum.namespace.bright_yellow(),
             spectrum.name.bright_yellow(),
             "[frequency]".bright_yellow(),
             "--help".bright_cyan());
    
    Ok(())
}

/// Render detailed help for a specific frequency
pub fn render_frequency_help(spectrum: &UVSpectrum, frequency: &str) -> Result<()> {
    // Find the specific wavelength
    let wavelength = spectrum.find_wavelength(frequency)
        .ok_or_else(|| anyhow::anyhow!("Frequency {} not found in prism {}", frequency, spectrum.name))?;
    
    println!("{}{}{} {} - {}", 
             spectrum.namespace.bright_yellow(),
             ":".bright_white(),
             spectrum.name.bright_yellow(), 
             wavelength.frequency.bold().bright_green(),
             wavelength.description.bright_white());
    println!("{}", "=".repeat(spectrum.namespace.len() + spectrum.name.len() + wavelength.frequency.len() + wavelength.description.len() + 5).bright_purple());
    println!();
    
    // Display input schema
    println!("{}", "Input parameters:".underline().bright_blue());
    render_schema_details(&wavelength.input.schema)?;
    
    // Display output schema
    println!();
    println!("{}", "Output format:".underline().bright_blue());
    render_schema_details(&wavelength.output.schema)?;
    
    // Display example usage
    println!();
    println!("{}", "Usage example:".underline().bright_blue());
    
    // Build an example command based on the input schema
    let example_command = build_example_command(spectrum, wavelength.frequency.as_str(), &wavelength.input.schema);
    println!("{}", example_command);
    
    Ok(())
}

/// Render schema details in a user-friendly format
fn render_schema_details(schema: &Value) -> Result<()> {
    let schema_type = schema.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
    
    match schema_type {
        "object" => {
            let properties = schema.get("properties").and_then(|p| p.as_object());
            let required = schema.get("required").and_then(|r| r.as_array())
                .map_or_else(Vec::new, |arr| {
                    arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                });
            
            if let Some(props) = properties {
                for (name, details) in props {
                    let is_required = required.contains(name);
                    let type_str = details.get("type").and_then(|t| t.as_str()).unwrap_or("any");
                    let description = details.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    
                    let req_marker = if is_required { "*".bright_red() } else { " ".normal() };
                    
                    println!("  {}{}: {}{} {}", 
                             req_marker,
                             name.bright_green(),
                             type_str.bright_cyan(),
                             if description.is_empty() { "".into() } else { " - ".normal() },
                             description.bright_white());
                    
                    // Print constraints if any
                    print_constraints(details);
                }
                
                if !required.is_empty() {
                    println!();
                    println!("  {} required parameter", "*".bright_red());
                }
            }
        },
        "array" => {
            let items = schema.get("items");
            println!("  {}: {}", "Array of".bright_white(), schema_type.bright_cyan());
            
            if let Some(items_schema) = items {
                println!("  Items schema:");
                render_schema_details(items_schema)?;
            }
        },
        _ => {
            println!("  {}: {}", "Type".bright_white(), schema_type.bright_cyan());
            
            // Print other properties if present
            if let Some(description) = schema.get("description").and_then(|d| d.as_str()) {
                println!("  {}: {}", "Description".bright_white(), description);
            }
            
            print_constraints(schema);
        }
    }
    
    Ok(())
}

/// Print schema constraints like minimum, maximum, etc.
fn print_constraints(schema: &Value) {
    let constraints = vec![
        ("minimum", "Min"),
        ("maximum", "Max"),
        ("minLength", "Min length"),
        ("maxLength", "Max length"),
        ("pattern", "Pattern"),
        ("format", "Format"),
        ("default", "Default"),
    ];
    
    for (json_key, display_name) in constraints {
        if let Some(value) = schema.get(json_key) {
            println!("    {}: {}", display_name.bright_white(), value);
        }
    }
    
    // Handle enum values
    if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
        let values: Vec<String> = enum_values.iter()
            .map(|v| format!("{}", v))
            .collect();
        println!("    {}: {}", "Allowed values".bright_white(), values.join(", ").bright_cyan());
    }
}

/// Build an example command based on schema
fn build_example_command(spectrum: &UVSpectrum, frequency: &str, schema: &Value) -> colored::ColoredString {
    let mut command = format!("uv {}:{} {}", spectrum.namespace, spectrum.name, frequency);
    
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        for (name, _) in properties {
            command.push_str(&format!(" --{} value", name));
        }
    }
    
    command.bright_green()
}

/// Scan for available prisms in the installation directory
fn scan_available_prisms() -> Result<Vec<UVSpectrumInfo>> {
    let mut prisms = Vec::new();
    
    // Get the installation directory
    let install_dir = get_install_dir()?;
    let prisms_dir = install_dir.join("prisms");
    
    if !prisms_dir.exists() {
        return Ok(prisms);
    }
    
    // Scan directories
    scan_prism_directory(&prisms_dir, &mut prisms)?;
    
    Ok(prisms)
}

/// Get the uv installation directory
fn get_install_dir() -> Result<PathBuf> {
    let home_dir = std::env::var("HOME")
        .context("HOME environment variable not set")?;
    
    let install_dir = std::env::var("UV_INSTALL_DIR")
        .unwrap_or(format!("{}/.uv", home_dir));
    
    Ok(PathBuf::from(install_dir))
}

/// Recursively scan a directory for prism spectrum files
fn scan_prism_directory(dir: &PathBuf, prisms: &mut Vec<UVSpectrumInfo>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // Scan subdirectories
            scan_prism_directory(&path, prisms)?;
        } else if path.file_name().map_or(false, |f| f == "spectrum.json") {
            // Found a spectrum.json file
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(spectrum) = serde_json::from_str::<UVSpectrum>(&content) {
                    prisms.push(UVSpectrumInfo {
                        namespace: spectrum.namespace.clone(),
                        name: spectrum.name.clone(),
                        description: spectrum.description.clone(),
                    });
                }
            }
        }
    }
    
    Ok(())
}

/// Group prisms by namespace
fn group_prisms_by_namespace(prisms: &[UVSpectrumInfo]) -> HashMap<String, Vec<UVSpectrumInfo>> {
    let mut map: HashMap<String, Vec<UVSpectrumInfo>> = HashMap::new();
    
    for prism in prisms {
        map.entry(prism.namespace.clone())
            .or_insert_with(Vec::new)
            .push(prism.clone());
    }
    
    map
}

/// Basic spectrum information for display
#[derive(Debug, Clone)]
struct UVSpectrumInfo {
    namespace: String,
    name: String,
    description: String,
}