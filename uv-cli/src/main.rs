use serde_json::{json, Value};
use uuid::Uuid;
use clap::Parser;

use uv_core::PrismMultiplexer;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Prism to execute in namespace:name format
    prism: String,

    /// Frequency to call (method)
    #[arg(default_value = "help")]
    frequency: String,

    /// Arguments for the frequency
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,

    /// Output raw JSON
    #[arg(long, default_value_t = false)]
    raw: bool,

    /// Show help for this prism
    #[arg(long, default_value_t = false)]
    help: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Create a multiplexer
    let multiplexer = PrismMultiplexer::new();
    
    if cli.help || cli.frequency == "help" {
        // TODO: Load the spectrum and show available frequencies
        println!("Help for prism: {}", cli.prism);
        // This will be enhanced later with actual spectrum info
        return Ok(());
    }
    
    // Connect to the specified prism
    let link = match multiplexer.establish_link(&cli.prism) {
        Ok(link) => link,
        Err(e) => {
            eprintln!("Failed to connect to prism {}: {}", cli.prism, e);
            return Err(Box::new(e));
        }
    };
    
    // Process arguments into a JSON object
    let args_json = process_args(&cli.args);
    
    // Send the command
    let request_id = Uuid::new_v4();
    if let Err(e) = link.send_wavefront(request_id, &cli.frequency, args_json) {
        eprintln!("Failed to send command: {}", e);
        return Err(Box::new(e));
    }
    
    // Absorb the response as raw Value
    match link.absorb::<Value>() {
        Ok(response) => {
            // Print raw or formatted JSON
            if cli.raw {
                match serde_json::to_string(&response) {
                    Ok(json) => println!("{}", json),
                    Err(e) => eprintln!("Error serializing response: {}", e),
                }
            } else {
                match serde_json::to_string_pretty(&response) {
                    Ok(json) => println!("{}", json),
                    Err(e) => eprintln!("Error serializing response: {}", e),
                }
            }
            Ok(())
        },
        Err(e) => {
            eprintln!("Error receiving response: {}", e);
            Err(Box::new(e))                
        }
    }
}

// Process arguments into a JSON object
fn process_args(args: &[String]) -> Value {
    let mut result = json!({});
    
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        
        if arg.starts_with("--") {
            let key = &arg[2..];
            
            // Check for --key=value format
            if let Some(eq_pos) = key.find('=') {
                let (real_key, value) = key.split_at(eq_pos);
                result[real_key] = json!(value[1..]);
            } 
            // Otherwise expect --key value format
            else if i + 1 < args.len() {
                result[key] = json!(args[i + 1]);
                i += 1; // Skip next arg since we used it as value
            } else {
                // Treat as boolean flag if no value
                result[key] = json!(true);
            }
        } else if i == 0 {
            // Special case for the first positional argument (subcommand)
            // We'll just ignore it as it's already captured as the frequency
        } else {
            // Add positional arguments with numeric keys
            result[format!("arg{}", i)] = json!(arg);
        }
        
        i += 1;
    }
    
    result
}
