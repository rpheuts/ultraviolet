mod server;
mod local;
mod remote;
mod parsing;
mod rendering;
mod interactive;

use std::ffi::OsString;
use anyhow::Result;

use parsing::cli_commands::match_cli_input;
use server::handle_server;
use local::handle_local;
use remote::handle_remote;
use interactive::{handle_interactive, handle_interactive_with_mode, ModeType};

#[tokio::main]
async fn main() -> Result<()> {
    // Check for standalone --help or -h
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && (args[1] == "--help" || args[1] == "-h") {
        return rendering::cli_help::render_global_help();
    }

    // Parse command line arguments
    let args = match_cli_input()?;
    let debug = args.get_flag("debug");
    let output = args.get_one::<String>("output");

    init_tracing(debug);

    // Check for --mode option to determine initial mode
    let initial_mode = args.get_one::<String>("mode").map(|mode_str| {
        match mode_str.as_str() {
            "chat" => ModeType::Chat,
            "cmd" => ModeType::Command,
            "normal" => ModeType::Prism,
            _ => ModeType::Prism, // Default fallback
        }
    });

    match args.subcommand() {
        Some(("server", sync_matches)) => {
            let bind_address: &String = sync_matches
                .get_one::<String>("address")
                .expect("No address and port specified");

            handle_server(bind_address.parse()?, sync_matches.get_flag("static"), sync_matches.get_flag("browser"), debug).await?;
        },
        Some(("cli", _cli_matches)) => {
            // Handle interactive CLI with optional mode
            if let Some(mode) = initial_mode {
                handle_interactive_with_mode(Some(mode))?;
            } else {
                handle_interactive()?;
            }
        },
        Some((external, sub_m)) => {
            let sub_args: Vec<String> = sub_m
                .get_many::<OsString>("")
                .unwrap_or_default()
                .filter_map(|s| s.to_str().map(|s| s.to_string()))
                .collect();

            // If the remote flag is provided, use handle_remote instead
            if let Some(remote_url) = args.get_one::<String>("remote") {
                handle_remote(remote_url, external, sub_args, output)?;
            } else {
                handle_local(external, sub_args, output)?;
            }
        }
        None => {
            // No subcommand provided - default to interactive mode
            if let Some(mode) = initial_mode {
                handle_interactive_with_mode(Some(mode))?;
            } else {
                handle_interactive()?;
            }
        }
    }
        
    Ok(())
}

fn init_tracing(debug: bool) {
    // Initialize tracing with filter based on debug/quiet flags
    let filter = if debug {
        "cli=debug,uv_service=debug"
    } else {
        "cli=info,uv_service=info"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
}
