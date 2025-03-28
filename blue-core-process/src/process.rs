use crate::error::{ProcessError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;
use nix::unistd::{setsid, Pid};
use nix::sys::signal::{self, Signal};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: PathBuf,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub id: String,
    pub pid: Option<u32>,
    pub started_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub exit_time: Option<DateTime<Utc>>,
    pub config: ProcessConfig,
}

pub struct Process {
    info: ProcessInfo,
    state_file: PathBuf,
    exit_file: PathBuf,
    #[allow(dead_code)] // Will be used for process locking in the future
    lock_file: PathBuf,
}

impl Process {
    pub fn new(mut config: ProcessConfig) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| ProcessError::StateError("HOME environment variable not set".into()))?;
        
        let state_dir = home.join(".blue/processes");
        std::fs::create_dir_all(&state_dir)
            .map_err(|e| ProcessError::StateError(format!("Failed to create state directory: {}", e)))?;

        // Create process-specific output directory
        let output_dir = home.join(".blue/processes/logs").join(&id);
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| ProcessError::StateError(format!("Failed to create output directory: {}", e)))?;
        config.output_dir = output_dir;

        Ok(Self {
            info: ProcessInfo {
                id: id.clone(),
                pid: None,
                started_at: None,
                exit_code: None,
                exit_time: None,
                config,
            },
            state_file: state_dir.join(format!("{}.json", id)),
            exit_file: state_dir.join(format!("{}.exit", id)),
            lock_file: state_dir.join(format!("{}.lock", id)),
        })
    }

    pub async fn spawn(&mut self) -> Result<()> {
        // Check if process is already running
        if self.is_running().await? {
            return Err(ProcessError::ProcessExists);
        }

        // Create output directory if it doesn't exist
        std::fs::create_dir_all(&self.info.config.output_dir)
            .map_err(|e| ProcessError::IoError(e))?;

        // Open output files
        let stdout_path = self.info.config.output_dir.join("stdout.log");
        let stderr_path = self.info.config.output_dir.join("stderr.log");
        let stdout = std::fs::File::create(&stdout_path)?;
        let stderr = std::fs::File::create(&stderr_path)?;

        // Create wrapper command that captures exit code
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
        .arg(format!(
            r#"
            # Run command with environment variables
            env {} sh -c '{}'  # Wrap command in sh -c to delay expansion
            exit_code=$?
            
            # Write exit code to file
            echo $exit_code > {}
            exit $exit_code
            "#,
            self.info.config.env.iter()
                .map(|(k, v)| format!("{}='{}'", k, v.replace("'", "'\\''"))).collect::<Vec<_>>().join(" "),
            if self.info.config.args.is_empty() {
                self.info.config.command.clone()
            } else {
                format!("{} {}", self.info.config.command, self.info.config.args.join(" "))
            },
            self.exit_file.display()
        ))
           .current_dir(&self.info.config.working_dir)
           .stdout(stdout)
           .stderr(stderr);

        // Use pre_exec to set up process group
        unsafe {
            cmd.pre_exec(|| {
                setsid().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(())
            });
        }

        // Spawn the process
        let child = cmd.spawn()
            .map_err(|e| ProcessError::StartError(e.to_string()))?;

        // Update process info
        self.info.pid = child.id();
        self.info.started_at = Some(Utc::now());
        self.info.exit_code = None;
        self.info.exit_time = None;

        // Save state
        self.save_state()?;

        Ok(())
    }

    pub async fn terminate(&mut self) -> Result<()> {
        if let Some(pid) = self.info.pid {
            let raw_pid = Pid::from_raw(pid as i32);
            
            // Get process group ID
            let pgid = match nix::unistd::getpgid(Some(raw_pid)) {
                Ok(pgid) => pgid,
                Err(nix::errno::Errno::ESRCH) => {
                    // Process already gone
                    self.info.exit_code = Some(0);
                    self.info.exit_time = Some(Utc::now());
                    self.info.pid = None;
                    self.save_state()?;
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Failed to get PGID for process {}: {}", pid, e);
                    return Ok(());
                }
            };
    
            // Send SIGTERM to process group
            if let Err(e) = signal::killpg(pgid, Signal::SIGTERM) {
                if e != nix::errno::Errno::ESRCH {
                    tracing::warn!("Failed to send SIGTERM to process group {}: {}", pgid, e);
                }
            }
    
            // Wait and check...
    
            // If still running, force kill process group
            if let Err(e) = signal::killpg(pgid, Signal::SIGKILL) {
                if e != nix::errno::Errno::ESRCH {
                    tracing::warn!("Failed to send SIGKILL to process group {}: {}", pgid, e);
                }
            }

            // Wait a bit for SIGKILL to take effect
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Get exit code from file or use 137 for SIGKILL
            self.info.exit_code = self.get_exit_code_from_file().or(Some(137));
            self.info.exit_time = Some(Utc::now());
            self.info.pid = None;
            self.save_state()?;
        }
        Ok(())
    }

    pub async fn remove(&mut self) -> Result<()> {
        // If process is running, stop it first
        if self.is_running().await? {
            self.terminate().await?;
        }

        // Remove state, exit, and lock files
        if let Err(e) = std::fs::remove_file(&self.state_file) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(ProcessError::StateError(format!("Failed to remove state file: {}", e)));
            }
        }

        if let Err(e) = std::fs::remove_file(&self.exit_file) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(ProcessError::StateError(format!("Failed to remove exit file: {}", e)));
            }
        }

        if let Err(e) = std::fs::remove_file(&self.lock_file) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(ProcessError::StateError(format!("Failed to remove lock file: {}", e)));
            }
        }

        // Remove output directory
        if let Err(e) = std::fs::remove_dir_all(&self.info.config.output_dir) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(ProcessError::StateError(format!("Failed to remove output directory: {}", e)));
            }
        }

        Ok(())
    }

    pub async fn is_running(&self) -> Result<bool> {
        if let Some(pid) = self.info.pid {
            let raw_pid = Pid::from_raw(pid as i32);
            match signal::kill(raw_pid, None) {
                Ok(_) => Ok(true),
                Err(nix::errno::Errno::ESRCH) => Ok(false),
                Err(e) => Err(ProcessError::StateError(format!("Failed to check process status: {}", e)))
            }
        } else {
            Ok(false)
        }
    }

    fn get_exit_code_from_file(&self) -> Option<i32> {
        if self.exit_file.exists() {
            if let Ok(code) = std::fs::read_to_string(&self.exit_file) {
                return code.trim().parse().ok();
            }
        }
        None
    }

    pub fn save_state(&mut self) -> Result<()> {
        // Check for exit code file before saving state
        if self.info.exit_code.is_none() {
            if let Some(code) = self.get_exit_code_from_file() {
                self.info.exit_code = Some(code);
                self.info.exit_time = Some(Utc::now());
                self.info.pid = None;
            }
        }

        let state = serde_json::to_string_pretty(&self.info)
            .map_err(|e| ProcessError::StateSaveError(e.to_string()))?;
        
        std::fs::write(&self.state_file, state)
            .map_err(|e| ProcessError::StateSaveError(e.to_string()))?;
        
        Ok(())
    }

    pub fn load_state(state_file: impl AsRef<Path>) -> Result<Self> {
        let state = std::fs::read_to_string(state_file.as_ref())
            .map_err(|e| ProcessError::StateLoadError(e.to_string()))?;
        
        let info: ProcessInfo = serde_json::from_str(&state)
            .map_err(|e| ProcessError::StateLoadError(e.to_string()))?;
        
        let state_dir = state_file.as_ref().parent()
            .ok_or_else(|| ProcessError::StateError("Invalid state file path".into()))?;
        
        Ok(Self {
            info: info.clone(),
            state_file: state_file.as_ref().to_path_buf(),
            exit_file: state_dir.join(format!("{}.exit", info.id)),
            lock_file: state_dir.join(format!("{}.lock", info.id)),
        })
    }

    pub fn id(&self) -> &str {
        &self.info.id
    }

    pub fn pid(&self) -> Option<u32> {
        self.info.pid
    }

    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.info.started_at
    }

    pub fn exit_code(&mut self) -> Option<i32> {
        // Check file first in case we haven't updated state yet
        if self.info.exit_code.is_none() {
            if let Some(code) = self.get_exit_code_from_file() {
                self.info.exit_code = Some(code);
                self.info.exit_time = Some(Utc::now());
                self.info.pid = None;
                // Save the updated state
                let _ = self.save_state();
            }
        }
        self.info.exit_code
    }

    pub fn exit_time(&self) -> Option<DateTime<Utc>> {
        self.info.exit_time
    }

    pub fn config(&self) -> &ProcessConfig {
        &self.info.config
    }

    // New methods for updating process state
    pub fn set_exit_code(&mut self, code: i32) {
        self.info.exit_code = Some(code);
    }

    pub fn set_exit_time(&mut self, time: DateTime<Utc>) {
        self.info.exit_time = Some(time);
    }

    pub fn clear_pid(&mut self) {
        self.info.pid = None;
    }
}
