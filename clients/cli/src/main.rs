mod server;
mod local;
mod parsing;
mod rendering;

use std::ffi::OsString;
use anyhow::Result;

use parsing::cli_commands::match_cli_input;
use server::handle_server;
use local::handle_local;

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

    match args.subcommand() {
        Some(("server", sync_matches)) => {
            let bind_address: &String = sync_matches
                .get_one::<String>("address")
                .expect("No address and port specified");

            handle_server(bind_address.parse()?, sync_matches.get_flag("static"), debug).await?;
        },
        Some((external, sub_m)) => {
            let args: Vec<String> = sub_m
                .get_many::<OsString>("")
                .unwrap_or_default()
                .filter_map(|s| s.to_str().map(|s| s.to_string()))
                .collect();

            handle_local(external, args, output)?;
        }
        _ => unreachable!()
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
