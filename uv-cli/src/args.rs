use std::path::PathBuf;
use clap::Parser;

/// Command line arguments for the UV CLI
#[derive(Parser, Debug)]
#[command(author, version, about = "Ultraviolet Command Line Interface")]
pub struct Cli {
    /// Prism to execute (namespace:name format)
    /// Only used when not in service mode
    #[arg(required_unless_present = "service")]
    pub prism: Option<String>,
    
    /// Frequency to call (method)
    /// Only used when not in service mode 
    #[arg(default_value = "help")]
    pub frequency: Option<String>,
    
    /// Arguments for the frequency
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
    
    /// Run in service mode (WebSocket server)
    #[arg(long)]
    pub service: bool,
    
    /// Connect to a remote service
    #[arg(long)]
    pub remote: Option<String>,
    
    /// Use secure WebSocket (wss://)
    #[arg(long)]
    pub secure: bool,
    
    /// Address to bind to when in service mode
    #[arg(long, default_value = "127.0.0.1:3000")]
    pub bind: String,
    
    /// Enable TLS for secure WebSocket connections in service mode
    #[arg(long)]
    pub tls: bool,
    
    /// Path to TLS certificate file (required if TLS is enabled)
    #[arg(long)]
    pub cert: Option<PathBuf>,
    
    /// Path to TLS key file (required if TLS is enabled)
    #[arg(long)]
    pub key: Option<PathBuf>,
    
    /// Serve static files from the specified directory
    #[arg(long)]
    pub static_dir: Option<PathBuf>,
    
    /// Output raw JSON
    #[arg(long, default_value_t = false)]
    pub raw: bool,
    
    /// Disable colored output
    #[arg(long, default_value_t = false)]
    pub no_color: bool,
    
    /// Enable verbose debug output
    #[arg(long, default_value_t = false)]
    pub debug: bool,
    
    /// Suppress all output except errors
    #[arg(long, default_value_t = false)]
    pub quiet: bool,
}
