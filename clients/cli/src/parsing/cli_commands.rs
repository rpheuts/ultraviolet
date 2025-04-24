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
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Set the output format (raw, pretty)")
                .default_value("pretty")
                .action(ArgAction::Set)
                
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
                    .default_value("127.0.0.1:3000")
                    .num_args(1),
            )
            .arg(
                Arg::new("static")
                    .long("no_static")
                    .help("Don't host the static files")
                    .action(ArgAction::SetFalse)
            )
        )
        .subcommand(
            Command::new("chat")
            .about("Start an interactive AI chat session using Bedrock LLMs")
            .arg(
                Arg::new("model")
                    .short('m')
                    .long("model")
                    .help("Specify the LLM model to use")
                    .action(ArgAction::Set)
                    .default_value("us.anthropic.claude-3-7-sonnet-20250219-v1:0")
            )
            .arg(
                Arg::new("max_tokens")
                    .long("max-tokens")
                    .help("Maximum number of tokens for the response")
                    .action(ArgAction::Set)
                    .default_value("4096")
            )
            .arg(
                Arg::new("context_file")
                    .short('c')
                    .long("context")
                    .help("Add a file as context for the AI")
                    .action(ArgAction::Append)
            )
        )
        .allow_external_subcommands(true)
        .get_matches())
}
