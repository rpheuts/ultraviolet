use std::path::PathBuf;
use serde_json::json;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use uuid::Uuid;
use clap::{Parser, Subcommand};

use uv_core::PrismMultiplexer;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a message to a prism and get the response
    Echo {
        /// Message to send
        #[arg(short, long)]
        message: String,
        
        /// Path to the prism library directory
        #[arg(short, long, default_value = "./target/release")]
        lib_path: PathBuf,
        
        /// Path to the spectrum directory
        #[arg(short, long, default_value = "./uv-prism-echo")]
        spectrum_path: PathBuf,
        
        /// Prism ID to connect to
        #[arg(short, long, default_value = "example:echo")]
        prism_id: String,
    },
}

fn main() {
    let cli = Cli::parse();
    
    // Create a runtime for async code
    let rt = Runtime::new().unwrap();
    
    match cli.command {
        Commands::Echo { message, lib_path: _, spectrum_path: _, prism_id } => {
            rt.block_on(async {
                // Create a multiplexer
                let multiplexer = PrismMultiplexer::new();
                
        // Note: We no longer need to add library paths or load prisms explicitly
        // as prisms are loaded on demand from the standard location
                
                // Connect to the echo prism
                let link = match multiplexer.establish_link(&prism_id).await {
                    Ok(link) => link,
                    Err(e) => {
                        eprintln!("Failed to connect to prism {}: {}", prism_id, e);
                        return;
                    }
                };
                
                // Create a message to send
                let payload = json!({
                    "message": message
                });
                
                // Send the message
                let request_id = Uuid::new_v4();
                if let Err(e) = link.send_wavefront(request_id, "echo", payload).await {
                    eprintln!("Failed to send message: {}", e);
                    return;
                }
                
                println!("Sent message: {}", message);
                
                // Wait for the response using absorb to collect all photons
                match link.absorb::<EchoOutput>().await {
                    Ok(output) => {
                        println!("Received response: {}", output.message);
                    },
                    Err(e) => {
                        eprintln!("Error receiving response: {}", e);
                    }
                }
            });
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoOutput {
    /// Echoed message
    pub message: String,
}