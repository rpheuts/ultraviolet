use clap::{Arg, ArgAction, ArgMatches, Command};
use anyhow::Result;

pub fn match_cli_input() -> Result<ArgMatches> {
    Ok(Command::new("uv")
        .about("Ultra-Violet CLI")
        .version("1.0.0")
        .subcommand_required(true)
        .disable_help_flag(true)  // Disable built-in help
        .disable_help_subcommand(true)  // Disable built-in help subcommand
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .help("Show help information")
                .action(ArgAction::SetTrue)
                .global(true)
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debug logging")
                .action(ArgAction::SetTrue)
                
        )
        .subcommand(
            Command::new("server")
            .short_flag('S')
            .long_flag("server")
            .about("Run Ultra-Violet in server mode")
            .arg(
                Arg::new("address")
                    .short('a')
                    .long("address")
                    .help("Local address and port to host UV on")
                    .action(ArgAction::Set)
                    .default_value("127.0.0.1:3000")
                    .num_args(1),
            )
        )
        .allow_external_subcommands(true)
        .get_matches())
}