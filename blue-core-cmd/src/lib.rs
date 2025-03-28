use blue_core::prelude::*;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use tokio::runtime::Runtime;
use std::collections::HashMap;

pub struct CoreCmdModule {
    manifest: ModuleManifest,
    runtime: Runtime,
}

impl CoreCmdModule {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;
        let lib_path = home.join(".blue/modules");
        let manifest = ModuleManifest::load(lib_path.join("blue").join("core-cmd").join("manifest.toml"))?;
        let runtime = Runtime::new()
            .map_err(|e| Error::Module(format!("Failed to create runtime: {}", e)))?;

        Ok(Self {
            manifest,
            runtime,
        })
    }

    fn handle_exec(&mut self, args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Missing required argument: command".into()))?;

        // Get optional arguments
        let shell = args.get("shell").and_then(|v| v.as_str()).unwrap_or("bash");
        let cwd = args.get("cwd").and_then(|v| v.as_str());
        let env_vars = args.get("env")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
                .collect::<HashMap<String, String>>())
            .unwrap_or_default();

        // Expand shell variables in command and cwd
        let expanded_command = shellexpand::full(command)
            .map_err(|e| Error::Module(format!("Failed to expand command: {}", e)))?
            .into_owned();
        let expanded_cwd = cwd.map(|c| shellexpand::full(c)
            .map_err(|e| Error::Module(format!("Failed to expand cwd: {}", e)))
            .map(|s| s.into_owned()))
            .transpose()?;

        // Build command
        let mut cmd = Command::new(shell);
        cmd.arg("-c").arg(&expanded_command);

        // Set working directory if provided
        if let Some(dir) = expanded_cwd {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        if stdout.is_some() || stderr.is_some() {
            // Streaming mode: pipe output directly
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let mut child = cmd.spawn()
                .map_err(|e| Error::Module(format!("Failed to spawn command: {}", e)))?;

            let stdout_pipe = child.stdout.take()
                .ok_or_else(|| Error::Module("Failed to capture stdout".into()))?;
            let stderr_pipe = child.stderr.take()
                .ok_or_else(|| Error::Module("Failed to capture stderr".into()))?;

            // Read both streams concurrently
            let stdout_reader = BufReader::new(stdout_pipe);
            let stderr_reader = BufReader::new(stderr_pipe);

            self.runtime.block_on(async {
                let stdout_lines = stdout_reader.lines();
                let stderr_lines = stderr_reader.lines();

                // Process stdout
                if let Some(w) = stdout {
                    for line in stdout_lines {
                        if let Ok(line) = line {
                            w.write_all(line.as_bytes())?;
                            w.write_all(b"\n")?;
                            w.flush()?;
                        }
                    }
                }

                // Process stderr
                if let Some(w) = stderr {
                    for line in stderr_lines {
                        if let Ok(line) = line {
                            w.write_all(line.as_bytes())?;
                            w.write_all(b"\n")?;
                            w.flush()?;
                        }
                    }
                }

                Ok::<(), Error>(())
            })?;

            // Wait for process to finish
            let status = child.wait()
                .map_err(|e| Error::Module(format!("Failed to wait for command: {}", e)))?;

            Ok(json!({
                "status": status.code().unwrap_or(-1)
            }))
        } else {
            // Non-streaming mode: collect output
            let output = cmd.output()
                .map_err(|e| Error::Module(format!("Failed to execute command: {}", e)))?;

            Ok(json!({
                "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
                "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
                "status": output.status.code().unwrap_or(-1)
            }))
        }
    }
}

impl Module for CoreCmdModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Validate method exists
        if self.manifest.find_method(path).is_none() {
            return Err(Error::MethodNotFound(path.join(" ")));
        }

        match path {
            ["exec"] => self.handle_exec(args, stdout, stderr),
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(CoreCmdModule::new().expect("Failed to create core-cmd module"))
}
