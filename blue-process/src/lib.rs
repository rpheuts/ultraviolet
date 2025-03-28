use blue_core::prelude::*;
use serde_json::json;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use humantime::format_duration;
use std::time::SystemTime;

pub struct ProcessModule {
    manifest: ModuleManifest,
    context: ModuleContext,
}

impl ProcessModule {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;
        let lib_path = home.join(".blue/modules");
        let manifest = ModuleManifest::load(lib_path.join("blue").join("process").join("manifest.toml"))?;
        let context = ModuleContext::new(lib_path);

        Ok(Self {
            manifest,
            context,
        })
    }

    fn find_process_by_prefix(&mut self, prefix: &str) -> Result<Value> {
        let processes = self.context.call_module("blue:core-process", &["list"], json!({}), None, None)?;
        
        if let Some(process_list) = processes.get("processes").and_then(|v| v.as_array()) {
            let matches: Vec<_> = process_list.iter()
                .filter(|p| p.get("id")
                    .and_then(|id| id.as_str())
                    .map(|id| id.starts_with(prefix))
                    .unwrap_or(false))
                .collect();

            match matches.len() {
                0 => Err(Error::Module(format!("No process found with ID prefix '{}'", prefix))),
                1 => Ok(matches[0].clone()),
                _ => Err(Error::Module(format!("Multiple processes found with ID prefix '{}'. Please be more specific.", prefix))),
            }
        } else {
            Err(Error::Module("Failed to get process list".into()))
        }
    }

    fn find_processes_by_prefixes(&mut self, prefixes: &[String]) -> Result<Vec<Value>> {
        let processes = self.context.call_module("blue:core-process", &["list"], json!({}), None, None)?;
        let mut result = Vec::new();
        
        if let Some(process_list) = processes.get("processes").and_then(|v| v.as_array()) {
            for prefix in prefixes {
                let matches: Vec<_> = process_list.iter()
                    .filter(|p| p.get("id")
                        .and_then(|id| id.as_str())
                        .map(|id| id.starts_with(prefix))
                        .unwrap_or(false))
                    .collect();

                match matches.len() {
                    0 => return Err(Error::Module(format!("No process found with ID prefix '{}'", prefix))),
                    1 => result.push(matches[0].clone()),
                    _ => return Err(Error::Module(format!("Multiple processes found with ID prefix '{}'. Please be more specific.", prefix))),
                }
            }
        }

        Ok(result)
    }

    fn find_processes_by_pattern(&mut self, pattern: &str) -> Result<Vec<Value>> {
        let processes = self.context.call_module("blue:core-process", &["list"], json!({}), None, None)?;
        
        if let Some(process_list) = processes.get("processes").and_then(|v| v.as_array()) {
            let matches: Vec<_> = process_list.iter()
                .filter(|p| p.get("config")
                    .and_then(|c| c.get("command"))
                    .and_then(|c| c.as_str())
                    .map(|cmd| cmd.contains(pattern))
                    .unwrap_or(false))
                .cloned()
                .collect();

            if matches.is_empty() {
                Err(Error::Module(format!("No processes found matching pattern '{}'", pattern)))
            } else {
                Ok(matches)
            }
        } else {
            Err(Error::Module("Failed to get process list".into()))
        }
    }

    fn find_running_processes(&mut self) -> Result<Vec<Value>> {
        let processes = self.context.call_module("blue:core-process", &["list"], json!({}), None, None)?;
        
        if let Some(process_list) = processes.get("processes").and_then(|v| v.as_array()) {
            let matches: Vec<_> = process_list.iter()
                .filter(|p| p.get("pid").and_then(|v| v.as_u64()).is_some())
                .cloned()
                .collect();

            if matches.is_empty() {
                Err(Error::Module("No running processes found".into()))
            } else {
                Ok(matches)
            }
        } else {
            Err(Error::Module("Failed to get process list".into()))
        }
    }

    fn format_relative_time(timestamp: &DateTime<Utc>) -> String {
        let now = SystemTime::now();
        let duration = now.duration_since(SystemTime::from(timestamp.clone()))
            .unwrap_or_default();
        format_duration(duration).to_string()
    }

    fn get_status(process: &Value) -> (&'static str, String) {
        if process.get("pid").and_then(|v| v.as_u64()).is_some() {
            ("Running", "Running".into())
        } else if let Some(code) = process.get("exit_code").and_then(|c| c.as_i64()) {
            if code == 0 {
                ("Success", "Success".into())
            } else {
                ("Failed", format!("Failed ({})", code))
            }
        } else {
            ("Unknown", "Unknown".into())
        }
    }

    fn handle_list(&mut self, args: Value) -> Result<Value> {
        // Get process list from core-process
        let processes = self.context.call_module("blue:core-process", &["list"], json!({}), None, None)?;
        
        let mut formatted_processes = Vec::new();
        
        if let Some(process_list) = processes.get("processes").and_then(|v| v.as_array()) {
            for process in process_list {
                // Get basic info
                let id = process.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let short_id = &id[..8.min(id.len())];
                
                // Get command
                let command = process.get("config")
                    .and_then(|c| c.get("command"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("unknown");

                // Get status and relative time
                let (status_key, status) = Self::get_status(process);
                let relative_time = process.get("started_at")
                    .and_then(|t| t.as_str())
                    .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                    .map(|dt| Self::format_relative_time(&dt.with_timezone(&Utc)))
                    .unwrap_or_else(|| "unknown".into());

                // Apply filters if any
                let status_filter = args.get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("all");
                
                let include = match status_filter {
                    "running" => status_key == "Running",
                    "exited" => status_key != "Running",
                    _ => true
                };

                if let Some(filter) = args.get("filter").and_then(|f| f.as_str()) {
                    if !command.contains(filter) {
                        continue;
                    }
                }

                if include {
                    formatted_processes.push(json!({
                        "short_id": short_id,
                        "pid": process.get("pid"),
                        "status": status,
                        "relative_time": relative_time,
                        "command": command
                    }));
                }
            }
        }

        Ok(json!({
            "processes": formatted_processes
        }))
    }

    fn handle_start(&mut self, args: Value) -> Result<Value> {
        // Extract command and arguments
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Missing command".into()))?;

        // Get working directory
        let working_dir = args.get("cwd")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from(".")));

        // Start the process
        self.context.call_module("blue:core-process", &["start"], json!({
            "command": command,
            "args": Vec::<String>::new(),
            "working_dir": working_dir,
            "evars": args.get("evars")  // Pass through evars array
        }), None, None)
    }

    fn handle_logs(&mut self, args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Get ID prefix from args
        let id_prefix = args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Missing process ID".into()))?;

        // Find the full process ID
        let process = self.find_process_by_prefix(id_prefix)?;
        let full_id = process.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Process missing ID".into()))?;

        // Check if we should follow
        let follow = args.get("follow")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if follow {
            // Use core-process follow command
            self.context.call_module("blue:core-process", &["follow"], json!({
                "id": full_id,
                "stream": args.get("stream").unwrap_or(&json!("both"))
            }), stdout, stderr)
        } else {
            // Use core-process logs command
            self.context.call_module("blue:core-process", &["logs"], json!({
                "id": full_id,
                "stream": args.get("stream").unwrap_or(&json!("both")),
                "lines": args.get("tail").unwrap_or(&json!(100))
            }), stdout, stderr)
        }
    }

    fn handle_stop(&mut self, args: Value) -> Result<Value> {
        let force = args.get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let processes = if let Some(ids) = args.get("ids").and_then(|v| v.as_array()) {
            // Stop processes by ID prefixes
            let id_strings: Vec<String> = ids.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect();
            self.find_processes_by_prefixes(&id_strings)?
        } else if let Some(pattern) = args.get("filter").and_then(|v| v.as_str()) {
            // Stop processes by command pattern
            self.find_processes_by_pattern(pattern)?
        } else if args.get("running").and_then(|v| v.as_bool()).unwrap_or(false) {
            // Stop all running processes
            self.find_running_processes()?
        } else {
            return Err(Error::Module("Must specify either ids, filter, or --running".into()));
        };

        let mut stopped = 0;
        for process in processes {
            let id = process.get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Module("Process missing ID".into()))?;

            self.context.call_module("blue:core-process", &["stop"], json!({
                "id": id,
                "force": force
            }), None, None)?;

            stopped += 1;
        }

        Ok(json!({
            "success": true,
            "stopped": stopped
        }))
    }
}

impl Module for ProcessModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        match path {
            ["list"] => self.handle_list(args),
            ["start"] => self.handle_start(args),
            ["logs"] => self.handle_logs(args, stdout, stderr),
            ["stop"] => self.handle_stop(args),
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(ProcessModule::new().expect("Failed to create process module"))
}
