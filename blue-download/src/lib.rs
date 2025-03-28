use blue_core::prelude::*;
use serde_json::json;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct DownloadStats {
    progress: f64,
    speed: f64,
    downloaded: u64,
    total: u64,
    elapsed: f64,
}

pub struct DownloadModule {
    manifest: ModuleManifest,
    context: ModuleContext,
}

impl DownloadModule {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;
        let lib_path = home.join(".blue/modules");
        let manifest = ModuleManifest::load(lib_path.join("blue").join("download").join("manifest.toml"))?;
        let context = ModuleContext::new(lib_path);

        Ok(Self {
            manifest,
            context,
        })
    }

    fn parse_progress(&self, logs: &Value) -> (String, String, String) {
        if let Some(lines) = logs.get("stdout").and_then(|v| v.as_array()) {
            if let Some(last_line) = lines.last() {
                if let Ok(stats) = serde_json::from_str::<DownloadStats>(last_line.as_str().unwrap_or("")) {
                    // Calculate remaining bytes
                    let remaining = stats.total - stats.downloaded;
                    
                    // Calculate ETA in seconds (remaining bytes / current speed)
                    let eta = if stats.speed > 0.0 {
                        remaining as f64 / stats.speed
                    } else {
                        0.0
                    };

                    return (
                        format!("{:.1}%", stats.progress),
                        format!("{:.1} MB/s", stats.speed / 1_000_000.0),
                        format!("{:.0}s", eta),
                    );
                }
            }
        }
        ("0%".into(), "0 B/s".into(), "unknown".into())
    }

    fn extract_url_from_command(command: &str) -> Option<String> {
        // Extract URL from: blue-module-runner --module-path ... --method get --args url=...
        let args_start = command.find("--args ")?;
        let args_str = &command[args_start + 7..];
        for pair in args_str.split(',') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "url" {
                    return Some(value.to_string());
                }
            }
        }
        None
    }

    fn format_download(&mut self, process: &Value) -> Result<Value> {
        let id = process.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let short_id = &id[..8.min(id.len())];

        // Get command to extract URL
        let url = process.get("config")
            .and_then(|c| c.get("command"))
            .and_then(|c| c.as_str())
            .and_then(|cmd| Self::extract_url_from_command(cmd))
            .unwrap_or_else(|| "unknown".to_string());

        // Get status and progress
        let status = if process.get("pid").and_then(|v| v.as_u64()).is_some() {
            "Downloading".into()
        } else if let Some(code) = process.get("exit_code").and_then(|c| c.as_i64()) {
            if code == 0 {
                "Complete".into()
            } else {
                format!("Failed ({})", code)
            }
        } else {
            "Unknown".into()
        };

        let (progress, speed, eta) = if status == "Downloading" {
            // Get progress from logs
            let logs = self.context.call_module("blue:core-process", &["logs"], json!({
                "id": id,
                "stream": "stdout",
                "lines": 100
            }), None, None)?;
            self.parse_progress(&logs)
        } else {
            if status == "Complete" {
                ("100%".into(), "-".into(), "-".into())
            } else {
                ("0%".into(), "-".into(), "-".into())
            }
        };

        Ok(json!({
            "id": short_id,
            "url": url,
            "status": status,
            "progress": progress,
            "speed": speed,
            "eta": eta
        }))
    }

    fn handle_list(&mut self) -> Result<Value> {
        // Get process list from core-process
        let processes = self.context.call_module("blue:core-process", &["list"], json!({}), None, None)?;
        let mut downloads = Vec::new();

        if let Some(process_list) = processes.get("processes").and_then(|v| v.as_array()) {
            for process in process_list {
                // Only include our download processes
                if let Some(command) = process.get("config")
                    .and_then(|c| c.get("command"))
                    .and_then(|c| c.as_str())
                {
                    if command.contains("blue-module-runner") && command.contains("core-download") {
                        downloads.push(self.format_download(process)?);
                    }
                }
            }
        }

        Ok(json!({
            "downloads": downloads
        }))
    }

    fn handle_start(&mut self, args: Value) -> Result<Value> {
        let url = args.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Missing URL".into()))?;

        // Get output directory
        let output_dir = args.get("output_dir")
            .and_then(|v| v.as_str())
            .map(|d| d.replace("~", &std::env::var("HOME").unwrap_or_else(|_| "~".into())))
            .unwrap_or_else(|| format!("{}/Downloads", std::env::var("HOME").unwrap_or_else(|_| "~".into())));

        // Create output directory if it doesn't exist
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| Error::Module(format!("Failed to create output directory: {}", e)))?;

        // Get path to core-download module
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;
        let module_path = home.join(".blue/modules/blue/core-download/module.dylib");

        // Start download process using module runner
        self.context.call_module("blue:core-process", &["start"], json!({
            "command": format!(
                "blue-module-runner --module-path {} --method get --args url={},output_dir={}",
                module_path.display(),
                url,
                output_dir
            ),
            "working_dir": "."
        }), None, None)?;

        // Return list of all downloads
        self.handle_list()
    }
}

impl Module for DownloadModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], args: Value, _stdout: Option<&mut dyn std::io::Write>, _stderr: Option<&mut dyn std::io::Write>) -> Result<Value> {
        match path {
            ["list"] => self.handle_list(),
            ["start"] => self.handle_start(args),
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(DownloadModule::new().expect("Failed to create download module"))
}
