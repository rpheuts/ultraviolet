use blue_core::prelude::*;
use clap::Parser;
use libloading::{Library, Symbol};
use std::io::{stdout, stderr};

#[derive(Parser)]
#[command(name = "blue-module-runner")]
struct Args {
    #[arg(long)]
    module_path: String,

    #[arg(long)]
    method: String,

    #[arg(long, default_value = "")]
    args: String,
}

fn parse_args(args_str: &str) -> Result<Value> {
    let mut map = serde_json::Map::new();
    if !args_str.is_empty() {
        for pair in args_str.split(',') {
            if let Some((key, value)) = pair.split_once('=') {
                map.insert(key.to_string(), Value::String(value.to_string()));
            }
        }
    }
    Ok(Value::Object(map))
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Load module
    let lib = unsafe { Library::new(&args.module_path) }
        .map_err(|e| Error::Module(format!("Failed to load module: {}", e)))?;
    let create_module: Symbol<fn() -> Box<dyn Module>> = unsafe {
        lib.get(b"create_module")
            .map_err(|e| Error::Module(format!("Failed to get create_module symbol: {}", e)))?
    };
    let mut module = create_module();

    // Parse method and args
    let method_parts: Vec<&str> = args.method.split(':').collect();
    let args = parse_args(&args.args)?;

    // Call module with stdout/stderr
    let mut stdout = stdout();
    let mut stderr = stderr();
    match module.call(
        &method_parts,
        args,
        Some(&mut stdout),
        Some(&mut stderr)
    ) {
        Ok(result) => {
            // Only print result if it's not null
            if !result.is_null() {
                println!("{}", serde_json::to_string(&result)
                    .map_err(|e| Error::Module(format!("Failed to serialize result: {}", e)))?);
            }
            Ok(())
        }
        Err(e) => Err(e)
    }
}
