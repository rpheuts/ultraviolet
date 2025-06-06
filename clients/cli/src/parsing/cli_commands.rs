use clap::{Arg, ArgAction, ArgMatches, Command};
use anyhow::Result;

pub fn match_cli_input() -> Result<ArgMatches> {
    Ok(Command::new("uv")
        .about("Ultra-Violet CLI")
        .version("1.0.0")
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
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Set the output format (raw, pretty)")
                .default_value("pretty")
                .action(ArgAction::Set)
                
        )
        .arg(
            Arg::new("remote")
                .long("remote")
                .help("WebSocket URL to a remote Ultraviolet server (e.g., ws://localhost:4000/ws)")
                .action(ArgAction::Set)
                .global(true)
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .help("Start in a specific interactive mode (chat, cmd, normal)")
                .action(ArgAction::Set)
                .value_parser(clap::builder::PossibleValuesParser::new(["chat", "cmd", "normal"]))
                .global(true)
        )
        .arg(
            Arg::new("upload")
                .long("upload")
                .help("Upload a local text file to remote storage (format: --upload <local_path> <remote_path>)")
                .action(ArgAction::Set)
                .num_args(2)
                .value_names(["LOCAL_FILE", "REMOTE_PATH"])
                .global(true)
        )
        .arg(
            Arg::new("download")
                .long("download")
                .help("Download a remote file to local storage (format: --download <remote_path> <local_path>)")
                .action(ArgAction::Set)
                .num_args(2)
                .value_names(["REMOTE_PATH", "LOCAL_FILE"])
                .global(true)
        )
        .arg(
            Arg::new("content_type")
                .long("content-type")
                .help("Content type for storage format (text/line, text/token, uv/photon)")
                .action(ArgAction::Set)
                .default_value("uv/photon")
                .global(true)
        )
        .subcommand(
            Command::new("server")
            .short_flag('s')
            .long_flag("server")
            .about("Run Ultra-Violet in server mode")
            .arg(
                Arg::new("address")
                    .short('a')
                    .long("address")
                    .help("Local address and port to host UV on")
                    .action(ArgAction::Set)
                    .default_value("127.0.0.1:4000")
                    .num_args(1),
            )
            .arg(
                Arg::new("static")
                    .long("no-static")
                    .help("Don't host the static files")
                    .action(ArgAction::SetFalse)
            )
            .arg(
                Arg::new("browser")
                    .long("no-browser")
                    .help("Don't attempt to open a browser window")
                    .action(ArgAction::SetFalse)
            )
        )
        .subcommand(
            Command::new("cli")
            .about("Start an interactive CLI for executing UV prism commands")
        )
        .allow_external_subcommands(true)
        .get_matches())
}
