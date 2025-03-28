use anyhow::Result;
use blue_core::{ModuleManifest, ModuleContext};
use blue_render_core::{Renderer, DisplayType, HelpData, HelpStyle, MethodHelp};
use blue_render_cli::CliRenderer;
use clap::Parser;
use serde_json::Value;
use std::path::PathBuf;
use ansi_term::Colour::{Blue, Yellow};
use ansi_term::Style;

mod args;
mod namespace;
use args::ArgProcessor;
use namespace::NamespaceResolver;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Module to execute (or 'help' to list available modules)
    #[arg(required_unless_present = "help")]
    module: Option<String>,

    /// All remaining arguments (method path and parameters)
    #[arg(trailing_var_arg = true)]
    remaining: Vec<String>,

    /// Show help for this module
    #[arg(long)]
    help: bool,

    /// Output format (default: formatted, options: raw, csv)
    #[arg(long, value_parser = ["formatted", "raw", "csv"], default_value = "formatted")]
    output: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Create module context
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|e| anyhow::anyhow!("Failed to get home directory: {}", e))?;
    let lib_path = home.join(".blue/modules");
    let mut context = ModuleContext::new(lib_path.clone());

    // Show help if requested
    if cli.help {
        if let Some(module) = cli.module {
            // Resolve namespace if not provided
            let resolver = NamespaceResolver::new(lib_path.clone());
            let resolved_module = resolver.resolve(&module)?;
            
            // Parse namespace:name format (now guaranteed to have namespace)
            let (namespace, name) = resolved_module.split_once(':').unwrap();
            
            // Load manifest from namespaced path
            let manifest_file = lib_path.join(namespace).join(name).join("manifest.toml");
            let manifest = ModuleManifest::load(manifest_file)?;
            
            // Convert manifest to help data
            let help_data = HelpData {
                name: format!("{}:{}", manifest.module.namespace, manifest.module.name),
                version: manifest.module.version.clone(),
                description: manifest.module.description.clone(),
                methods: manifest.methods.iter().map(|m| MethodHelp {
                    path: m.path.clone(),
                    description: m.description.clone(),
                    args: m.args_schema.clone(),
                    returns: m.return_schema.clone(),
                    display: m.display.clone(),
                }).collect(),
            };

            // Render help using the renderer
            let renderer = CliRenderer::new();
            let display = DisplayType::Help { 
                style: HelpStyle::Detailed 
            };
            println!("{}", renderer.render(&serde_json::to_value(help_data)?, &display)?);
        } else {
            // List available modules by scanning namespaces
            println!("\n{}", Blue.bold().paint("Available modules:"));
            
            // Scan each namespace directory
            for namespace_entry in std::fs::read_dir(&lib_path)? {
                let namespace_entry = namespace_entry?;
                if namespace_entry.file_type()?.is_dir() {
                    let namespace = namespace_entry.file_name();
                    
                    // Print namespace header
                    println!("\n{}/", Yellow.paint(namespace.to_string_lossy()));
                    
                    // List modules in namespace
                    for module_entry in std::fs::read_dir(namespace_entry.path())? {
                        let module_entry = module_entry?;
                        if module_entry.file_type()?.is_dir() {
                            let module_name = module_entry.file_name();
                            println!("  {}", Style::new().paint(module_name.to_string_lossy()));
                        }
                    }
                }
            }
            println!("\n{}", Style::new().dimmed().paint("Use 'blue namespace:module --help' for module-specific help"));
        }
        return Ok(());
    }

    // Load module manifest
    let module = cli.module.as_ref().unwrap();  // Safe because required_unless_present = "help"
    
    // Resolve namespace if not provided
    let resolver = NamespaceResolver::new(lib_path.clone());
    let resolved_module = resolver.resolve(module)?;
    
    // Parse namespace:name format (now guaranteed to have namespace)
    let (namespace, name) = resolved_module.split_once(':').unwrap();
    
    // Load manifest from namespaced path
    let manifest_file = lib_path.join(namespace).join(name).join("manifest.toml");
    let manifest = ModuleManifest::load(manifest_file)?;

    // Verify namespace and name match manifest
    if manifest.module.namespace != namespace || manifest.module.name != name {
        anyhow::bail!(
            "Module namespace/name mismatch. Expected {}/{}, got {}/{}",
            namespace, name, manifest.module.namespace, manifest.module.name
        );
    }

    // Split remaining args into method path and arguments
    let mut method_path = Vec::new();
    let mut args_start = 0;

    // First try to find a valid method path
    'outer: for len in 1..=cli.remaining.len() {
        let potential_path: Vec<&str> = cli.remaining[..len].iter().map(|s| s.as_str()).collect();
        if manifest.find_method(&potential_path).is_some() {
            method_path = potential_path;
            args_start = len;
            break 'outer;
        }
    }

    if method_path.is_empty() {
        anyhow::bail!("No valid method found in command");
    }

    // Get the method's schema from manifest
    let method = manifest.find_method(&method_path)
        .ok_or_else(|| anyhow::anyhow!("Method not found: {}", method_path.join(" ")))?;

    // Process arguments
    let mut processor = ArgProcessor::new(method.args_schema.clone().unwrap_or(Value::Null));
    let mut i = args_start;
    while i < cli.remaining.len() {
        let arg = &cli.remaining[i];
        if arg.starts_with("--") {
            let name = &arg[2..];
            if processor.is_boolean_param(name) {
                processor.add_flag(name);
                i += 1;
            } else {
                i += 1;
                if i < cli.remaining.len() {
                    processor.add_named_arg(name, &cli.remaining[i]);
                    i += 1;
                } else {
                    anyhow::bail!("Missing value for argument: {}", name);
                }
            }
        } else {
            processor.add_positional_arg(arg);
            i += 1;
        }
    }

    let args = processor.process()?;

    // Determine display type from config or output flag
    let display_type = match cli.output.as_str() {
        "raw" => DisplayType::Raw,
        "csv" => {
            // Only convert table displays to CSV
            if let Some(display) = &method.display {
                if let Some(DisplayType::Table { source, columns, style: _, format: _ }) = DisplayType::from_display_config(display) {
                    DisplayType::Csv { source: source.clone(), columns: columns.clone() }
                } else {
                    // Non-table displays fall back to raw
                    DisplayType::Raw
                }
            } else {
                DisplayType::Raw
            }
        }
        _ => {
            // Default formatted display
            if let Some(display) = &method.display {
                DisplayType::from_display_config(display)
                    .unwrap_or(DisplayType::Raw)
            } else {
                DisplayType::Raw
            }
        }
    };

    // For stream display type, provide stdout and stderr streams
    let result = if matches!(display_type, DisplayType::Stream { .. }) {
        let mut stdout = std::io::stdout();
        let mut stderr = std::io::stderr();
        context.call_module(&resolved_module, &method_path, args, Some(&mut stdout), Some(&mut stderr))?
    } else {
        context.call_module(&resolved_module, &method_path, args, None, None)?
    };

    // Render the result
    let renderer = CliRenderer::new();
    println!("{}", <CliRenderer as Renderer>::render(&renderer, &result, &display_type)?);

    Ok(())
}
