mod error;
mod process;
mod output;

pub use error::{ProcessError, Result};
pub use process::{Process, ProcessConfig, ProcessInfo};
pub use output::{Stream, OutputReader, OutputRotator};

use blue_core::prelude::*;
use serde_json::json;
use std::path::PathBuf;
use std::collections::HashMap;
use chrono::Utc;

pub struct CoreProcessModule {
    manifest: ModuleManifest,
}

impl CoreProcessModule {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| ProcessError::StateError(format!("Failed to get home directory: {}", e)))?;
        let lib_path = home.join(".blue/modules");
        let manifest = ModuleManifest::load(lib_path.join("blue").join("core-process").join("manifest.toml"))?;
        Ok(Self { manifest })
    }

    async fn update_process_states(&self) -> Result<()> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| ProcessError::StateError("HOME environment variable not set".into()))?;
        let state_dir = home.join(".blue/processes");
        
        for entry in std::fs::read_dir(&state_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(mut process) = Process::load_state(&path) {
                    // Only check processes without exit codes
                    if process.exit_code().is_none() && !process.is_running().await? {
                        // Process is no longer running but we don't have an exit code
                        // This can happen if the process was killed externally
                        process.set_exit_code(0);
                        process.set_exit_time(Utc::now());
                        process.clear_pid();
                        process.save_state()?;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_start(&self, args: Value) -> Result<Value> {
        // Extract command and arguments
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProcessError::StateError("Missing command".into()))?;

        let cmd_args = args.get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect::<Vec<_>>())
            .unwrap_or_default();

        // Get working directory
        let working_dir = args.get("working_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from(".")));

        // Get environment variables
        let env_vars = args.get("evars")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| {
                    let parts: Vec<_> = s.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        Some((parts[0].to_string(), parts[1].to_string()))
                    } else {
                        None
                    }
                })
                .collect::<HashMap<_, _>>())
            .unwrap_or_default();

        // Create process config
        let config = ProcessConfig {
            command: command.to_string(),
            args: cmd_args,
            env: env_vars,
            working_dir,
            output_dir: PathBuf::new(), // Will be set by Process::new
        };

        // Create and spawn process
        let mut process = Process::new(config)?;
        tokio::runtime::Runtime::new()
            .map_err(|e| ProcessError::StateError(format!("Failed to create runtime: {}", e)))?
            .block_on(process.spawn())?;

        Ok(json!({
            "id": process.id(),
            "pid": process.pid()
        }))
    }

    fn handle_stop(&self, args: Value) -> Result<Value> {
        let id = args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProcessError::StateError("Missing process ID".into()))?;

        // Find process state file
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| ProcessError::StateError("HOME environment variable not set".into()))?;
        let state_file = home.join(".blue/processes").join(format!("{}.json", id));

        if !state_file.exists() {
            return Err(ProcessError::ProcessNotFound);
        }

        // Load and stop process
        let mut process = Process::load_state(&state_file)?;
        tokio::runtime::Runtime::new()
            .map_err(|e| ProcessError::StateError(format!("Failed to create runtime: {}", e)))?
            .block_on(process.terminate())?;

        Ok(json!({
            "success": true,
            "exit_code": process.exit_code()
        }))
    }

    fn handle_remove(&self, args: Value) -> Result<Value> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| ProcessError::StateError("HOME environment variable not set".into()))?;
        let state_dir = home.join(".blue/processes");

        // Handle --all flag
        if args.get("all").and_then(|v| v.as_bool()).unwrap_or(false) {
            let mut removed_count = 0;
            let runtime = tokio::runtime::Runtime::new()
                .map_err(|e| ProcessError::StateError(format!("Failed to create runtime: {}", e)))?;

            for entry in std::fs::read_dir(&state_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    if let Ok(mut process) = Process::load_state(&path) {
                        // Only remove if process has exited
                        if process.exit_code().is_some() {
                            runtime.block_on(process.remove())?;
                            removed_count += 1;
                        }
                    }
                }
            }

            Ok(json!({
                "success": true,
                "removed_count": removed_count
            }))
        } else {
            // Handle single process removal
            let id = args.get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ProcessError::StateError("Missing process ID".into()))?;

            let state_file = state_dir.join(format!("{}.json", id));

            if !state_file.exists() {
                return Err(ProcessError::ProcessNotFound);
            }

            // Load and remove process
            let mut process = Process::load_state(&state_file)?;
            tokio::runtime::Runtime::new()
                .map_err(|e| ProcessError::StateError(format!("Failed to create runtime: {}", e)))?
                .block_on(process.remove())?;

            Ok(json!({
                "success": true,
                "removed_count": 1
            }))
        }
    }

    fn handle_logs(&self, args: Value) -> Result<Value> {
        let id = args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProcessError::StateError("Missing process ID".into()))?;

        let stream = args.get("stream")
            .and_then(|v| v.as_str())
            .unwrap_or("both");

        let lines = args.get("lines")
            .and_then(|v| v.as_u64())  // Try to get as u64 first
            .or_else(|| {  // If that fails, try parsing from string
                args.get("lines")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
            })
            .unwrap_or(100) as usize;

        // Find process state file
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| ProcessError::StateError("HOME environment variable not set".into()))?;
        let state_file = home.join(".blue/processes").join(format!("{}.json", id));

        if !state_file.exists() {
            return Err(ProcessError::ProcessNotFound);
        }

        // Load process to get output directory
        let process = Process::load_state(&state_file)?;
        let reader = OutputReader::new(&process.config().output_dir);
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| ProcessError::StateError(format!("Failed to create runtime: {}", e)))?;

        let mut result = json!({});
        let obj = result.as_object_mut().unwrap();

        // Read requested streams
        match stream {
            "stdout" | "both" => {
                let stdout = runtime.block_on(reader.read_lines(Stream::Stdout, lines))?;
                obj.insert("stdout".to_string(), json!(stdout));
            }
            _ => {}
        }

        match stream {
            "stderr" | "both" => {
                let stderr = runtime.block_on(reader.read_lines(Stream::Stderr, lines))?;
                obj.insert("stderr".to_string(), json!(stderr));
            }
            _ => {}
        }

        Ok(result)
    }

    fn handle_follow(&self, args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        let id = args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProcessError::StateError("Missing process ID".into()))?;

        let stream = args.get("stream")
            .and_then(|v| v.as_str())
            .unwrap_or("both");

        // Find process state file
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| ProcessError::StateError("HOME environment variable not set".into()))?;
        let state_file = home.join(".blue/processes").join(format!("{}.json", id));

        if !state_file.exists() {
            return Err(ProcessError::ProcessNotFound);
        }

        // Load process to get output directory
        let process = Process::load_state(&state_file)?;
        let reader = OutputReader::new(&process.config().output_dir);
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| ProcessError::StateError(format!("Failed to create runtime: {}", e)))?;

        // Follow requested streams
        match stream {
            "stdout" | "both" => {
                if let Some(stdout) = stdout {
                    runtime.block_on(reader.follow(Stream::Stdout, stdout))?;
                }
            }
            _ => {}
        }

        match stream {
            "stderr" | "both" => {
                if let Some(stderr) = stderr {
                    runtime.block_on(reader.follow(Stream::Stderr, stderr))?;
                }
            }
            _ => {}
        }

        Ok(json!({ "success": true }))
    }

    fn handle_list(&self) -> Result<Value> {
        // Update process states first
        tokio::runtime::Runtime::new()
            .map_err(|e| ProcessError::StateError(format!("Failed to create runtime: {}", e)))?
            .block_on(self.update_process_states())?;

        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| ProcessError::StateError(format!("Failed to get home directory: {}", e)))?;
        
        let state_dir = home.join(".blue/processes");
        if !state_dir.exists() {
            return Ok(json!({ "processes": [] }));
        }

        let mut processes = Vec::new();
        for entry in std::fs::read_dir(&state_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(mut process) = Process::load_state(&path) {
                    // Get all the info we need first
                    let id = process.id().to_string();
                    let pid = process.pid();
                    let started_at = process.started_at();
                    let exit_code = process.exit_code();
                    let exit_time = process.exit_time();
                    let config = process.config();
                    let mut cmd = config.command.clone();
                    if !config.args.is_empty() {
                        cmd = format!("{} {}", cmd, config.args.join(" "));
                    }

                    processes.push(json!({
                        "id": id,
                        "pid": pid,
                        "started_at": started_at,
                        "exit_code": exit_code,
                        "exit_time": exit_time,
                        "config": {
                            "command": cmd,
                            "working_dir": config.working_dir,
                            "output_dir": config.output_dir
                        }
                    }));
                }
            }
        }

        Ok(json!({ "processes": processes }))
    }
}

impl Module for CoreProcessModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> std::result::Result<Value, Error> {
        match path {
            ["start"] => self.handle_start(args).map_err(|e| Error::Module(e.to_string())),
            ["stop"] => self.handle_stop(args).map_err(|e| Error::Module(e.to_string())),
            ["remove"] => self.handle_remove(args).map_err(|e| Error::Module(e.to_string())),
            ["logs"] => self.handle_logs(args).map_err(|e| Error::Module(e.to_string())),
            ["follow"] => self.handle_follow(args, stdout, stderr).map_err(|e| Error::Module(e.to_string())),
            ["list"] => self.handle_list().map_err(|e| Error::Module(e.to_string())),
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(CoreProcessModule::new().expect("Failed to create core-process module"))
}
