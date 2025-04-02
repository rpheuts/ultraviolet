# Process Management System Design

## Overview

The Process Management System enables Ultraviolet to run and manage long-running processes without requiring a daemon. This document describes the architecture and implementation details of this system, which can be integrated with the prism-based architecture.

## Core Concepts

### Process

A process represents an external command execution with:
- Unique identifier (UUID)
- Command and arguments
- Environment variables
- Working directory
- Output capture
- State persistence

### Process State

Process state is persisted to the filesystem and includes:
- Process ID (PID)
- Start time
- Exit code (if terminated)
- Exit time (if terminated)
- Configuration details

### Process Output

Process output is captured and managed with:
- Separate stdout and stderr streams
- Log rotation capabilities
- Historical and live access

## Architecture

The process management system consists of three main components:

```
┌───────────────────────────────────────────────────────────┐
│                   Process Module (API)                    │
│   User-friendly interface for process management          │
└───────────────────────────┬───────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────┐
│                Core Process Module (Engine)               │
│   Low-level process management implementation             │
└───────────────────────────┬───────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────┐
│                    Module Runner                          │
│   Lightweight process execution environment               │
└───────────────────────────────────────────────────────────┘
```

### Core Process Module

The Core Process Module (`blue-core-process`) provides the fundamental implementation of process management:

1. **Process Creation and Lifecycle**
   - Spawns processes with unique identifiers
   - Tracks process state through filesystem
   - Manages process termination and cleanup

2. **State Management**
   - Persists process information in JSON files
   - Captures exit codes in separate files
   - Recovers from unexpected terminations

3. **Output Management**
   - Captures stdout and stderr to log files
   - Supports log rotation and retention policies
   - Provides access to historical and live output

### Process Module

The Process Module (`blue-process`) provides a user-friendly interface:

1. **Command Interface**
   - `list`: Shows running and completed processes
   - `start`: Launches new processes
   - `stop`: Terminates running processes
   - `logs`: Accesses process output

2. **Process Identification**
   - Supports ID prefix matching
   - Command pattern filtering
   - Status-based filtering

3. **Output Formatting**
   - Human-readable status information
   - Relative time formatting
   - Structured output for programmatic use

### Module Runner

The Module Runner (`blue-module-runner`) provides a lightweight execution environment:

1. **Dynamic Module Loading**
   - Loads modules at runtime
   - Passes arguments and handles I/O
   - Enables recursive process execution

2. **Minimal Overhead**
   - Avoids CLI initialization costs
   - Direct module method invocation
   - Efficient I/O handling

## Implementation Details

### Process Lifecycle

#### Creation

1. Generate a unique UUID for the process
2. Create state and output directories
3. Construct a shell command wrapper that:
   - Sets up environment variables
   - Executes the target command
   - Captures the exit code
4. Use `setsid()` to create a new process group
5. Redirect stdout/stderr to log files
6. Store PID and start time in state file

```rust
pub async fn spawn(&mut self) -> Result<()> {
    // Create output directory
    std::fs::create_dir_all(&self.info.config.output_dir)?;

    // Open output files
    let stdout = std::fs::File::create(self.info.config.output_dir.join("stdout.log"))?;
    let stderr = std::fs::File::create(self.info.config.output_dir.join("stderr.log"))?;

    // Create wrapper command
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
       .arg(format!(
           r#"
           # Run command with environment variables
           env {} sh -c '{}'
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
    let child = cmd.spawn()?;

    // Update process info
    self.info.pid = child.id();
    self.info.started_at = Some(Utc::now());
    self.save_state()?;

    Ok(())
}
```

#### Monitoring

1. Check process existence using signals
2. Update state when processes terminate
3. Maintain exit codes and termination times

```rust
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
```

#### Termination

1. Send SIGTERM to process group for graceful shutdown
2. Fall back to SIGKILL if process doesn't terminate
3. Update state file with exit code and time

```rust
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
                return Err(ProcessError::TerminationError(format!("Failed to get PGID: {}", e)));
            }
        };

        // Send SIGTERM to process group
        if let Err(e) = signal::killpg(pgid, Signal::SIGTERM) {
            if e != nix::errno::Errno::ESRCH {
                return Err(ProcessError::TerminationError(format!("Failed to send SIGTERM: {}", e)));
            }
        }

        // Wait and check...

        // If still running, force kill process group
        if let Err(e) = signal::killpg(pgid, Signal::SIGKILL) {
            if e != nix::errno::Errno::ESRCH {
                return Err(ProcessError::TerminationError(format!("Failed to send SIGKILL: {}", e)));
            }
        }

        // Update state
        self.info.exit_code = self.get_exit_code_from_file().or(Some(137));
        self.info.exit_time = Some(Utc::now());
        self.info.pid = None;
        self.save_state()?;
    }
    Ok(())
}
```

#### Cleanup

1. Remove state files for completed processes
2. Clean up log files if requested

```rust
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

    // Remove output directory
    if let Err(e) = std::fs::remove_dir_all(&self.info.config.output_dir) {
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(ProcessError::StateError(format!("Failed to remove output directory: {}", e)));
        }
    }

    Ok(())
}
```

### State Persistence

Process state is persisted to the filesystem using JSON files:

1. **State File**: `~/.blue/processes/{uuid}.json`
   - Contains process metadata and configuration
   - Updated on process creation, termination, and status changes

2. **Exit File**: `~/.blue/processes/{uuid}.exit`
   - Contains the process exit code
   - Written by the wrapper script on process completion

3. **Lock File**: `~/.blue/processes/{uuid}.lock`
   - Reserved for future use (process locking)

```rust
pub fn save_state(&mut self) -> Result<()> {
    // Check for exit code file before saving state
    if self.info.exit_code.is_none() {
        if let Some(code) = self.get_exit_code_from_file() {
            self.info.exit_code = Some(code);
            self.info.exit_time = Some(Utc::now());
            self.info.pid = None;
        }
    }

    let state = serde_json::to_string_pretty(&self.info)?;
    std::fs::write(&self.state_file, state)?;
    
    Ok(())
}

pub fn load_state(state_file: impl AsRef<Path>) -> Result<Self> {
    let state = std::fs::read_to_string(state_file.as_ref())?;
    let info: ProcessInfo = serde_json::from_str(&state)?;
    
    let state_dir = state_file.as_ref().parent()
        .ok_or_else(|| ProcessError::StateError("Invalid state file path".into()))?;
    
    Ok(Self {
        info: info.clone(),
        state_file: state_file.as_ref().to_path_buf(),
        exit_file: state_dir.join(format!("{}.exit", info.id)),
        lock_file: state_dir.join(format!("{}.lock", info.id)),
    })
}
```

### Output Management

Process output is captured and managed:

1. **Output Files**:
   - `~/.blue/processes/logs/{uuid}/stdout.log`
   - `~/.blue/processes/logs/{uuid}/stderr.log`

2. **Output Reading**:
   - Historical access with line limits
   - Live following with streaming

3. **Log Rotation**:
   - Size-based rotation
   - Configurable retention policy

```rust
pub async fn read_lines(&self, stream: Stream, lines: usize) -> Result<Vec<String>> {
    let path = self.output_dir.join(stream.filename());
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(&path).await?;
    let reader = BufReader::new(file);
    let mut lines_vec = Vec::new();
    let mut lines_iter = reader.lines();

    while let Some(line) = lines_iter.next_line().await? {
        lines_vec.push(line);
    }

    // Keep only the last N lines
    if lines_vec.len() > lines {
        lines_vec = lines_vec.split_off(lines_vec.len() - lines);
    }

    Ok(lines_vec)
}

pub async fn follow(&self, stream: Stream, writer: &mut dyn Write) -> Result<()> {
    let path = self.output_dir.join(stream.filename());
    if !path.exists() {
        return Ok(());
    }

    let file = File::open(&path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        writeln!(writer, "{}", line)?;
        writer.flush()?;
    }

    Ok(())
}
```

## Integration with Prism Architecture

The process management system can be integrated with the prism architecture:

### Process Prism

A specialized prism that manages external processes:

```rust
struct ProcessPrism {
    core: PrismCore,
    processes: HashMap<String, Process>,
}

#[async_trait]
impl UVPrism for ProcessPrism {
    async fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.core = PrismCore::new(spectrum, Arc::clone(&GLOBAL_MULTIPLEXER));
        Ok(())
    }
    
    async fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        match pulse {
            UVPulse::Wavefront(wavefront) => {
                match wavefront.frequency.as_str() {
                    "start" => {
                        // Extract process config from wavefront
                        let config = self.extract_process_config(&wavefront.input)?;
                        
                        // Create and spawn process
                        let mut process = Process::new(config)?;
                        process.spawn().await?;
                        
                        // Store process
                        self.processes.insert(process.id().to_string(), process);
                        
                        // Return process ID
                        link.emit_photon(id, json!({ "id": process.id() })).await?;
                        link.emit_trap(id, None).await?;
                        
                        Ok(true)
                    },
                    "stop" => {
                        // Extract process ID from wavefront
                        let process_id = wavefront.input.get("id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| UVError::InvalidInput("Missing process ID".into()))?;
                        
                        // Find and stop process
                        if let Some(mut process) = self.processes.get_mut(process_id) {
                            process.terminate().await?;
                            link.emit_photon(id, json!({ "success": true })).await?;
                        } else {
                            link.emit_trap(id, Some(UVError::NotFound("Process not found".into()))).await?;
                        }
                        
                        Ok(true)
                    },
                    // Other methods...
                    _ => Ok(false)
                }
            },
            UVPulse::Extinguish => {
                // Terminate all processes on shutdown
                for (_, mut process) in self.processes.iter_mut() {
                    if process.is_running().await? {
                        let _ = process.terminate().await;
                    }
                }
                Ok(true)
            },
            _ => Ok(false)
        }
    }
}
```

### Process Output Streaming

Process output can be streamed using photon sequences:

```rust
async fn stream_output(&self, process_id: &str, link: &UVLink, request_id: Uuid) -> Result<()> {
    if let Some(process) = self.processes.get(process_id) {
        let reader = OutputReader::new(&process.config().output_dir);
        
        // Create a task to stream output
        tokio::spawn(async move {
            let mut stdout_file = File::open(reader.output_path(Stream::Stdout)).await?;
            let mut buf = [0; 4096];
            
            loop {
                match stdout_file.read(&mut buf).await {
                    Ok(0) => {
                        // End of file, wait and try again
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    },
                    Ok(n) => {
                        // Send data as photon
                        let data = String::from_utf8_lossy(&buf[0..n]).to_string();
                        link.emit_photon(request_id, json!({ "data": data })).await?;
                    },
                    Err(e) => {
                        // Send error and break
                        link.emit_trap(request_id, Some(UVError::IoError(e.to_string()))).await?;
                        break;
                    }
                }
                
                // Check if process is still running
                if !process.is_running().await? {
                    // Send final exit code
                    link.emit_photon(request_id, json!({ 
                        "exit_code": process.exit_code() 
                    })).await?;
                    link.emit_trap(request_id, None).await?;
                    break;
                }
            }
            
            Ok::<(), UVError>(())
        });
    }
    
    Ok(())
}
```

### Process Management Spectrum

The process management capabilities can be described in a spectrum:

```json
{
  "name": "process",
  "version": "1.0.0",
  "frequencies": [
    {
      "name": "start",
      "description": "Start a new process",
      "input_schema": {
        "type": "object",
        "properties": {
          "command": { "type": "string" },
          "args": { "type": "array", "items": { "type": "string" } },
          "env": { "type": "object" },
          "working_dir": { "type": "string" }
        },
        "required": ["command"]
      },
      "output_schema": {
        "type": "object",
        "properties": {
          "id": { "type": "string" }
        }
      }
    },
    {
      "name": "stop",
      "description": "Stop a running process",
      "input_schema": {
        "type": "object",
        "properties": {
          "id": { "type": "string" },
          "force": { "type": "boolean" }
        },
        "required": ["id"]
      },
      "output_schema": {
        "type": "object",
        "properties": {
          "success": { "type": "boolean" }
        }
      }
    },
    {
      "name": "list",
      "description": "List processes",
      "input_schema": {
        "type": "object",
        "properties": {
          "status": { "type": "string", "enum": ["all", "running", "exited"] },
          "filter": { "type": "string" }
        }
      },
      "output_schema": {
        "type": "object",
        "properties": {
          "processes": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "id": { "type": "string" },
                "pid": { "type": "number" },
                "status": { "type": "string" },
                "command": { "type": "string" },
                "started_at": { "type": "string" },
                "exit_code": { "type": "number" }
              }
            }
          }
        }
      }
    },
    {
      "name": "logs",
      "description": "Get process logs",
      "input_schema": {
        "type": "object",
        "properties": {
          "id": { "type": "string" },
          "stream": { "type": "string", "enum": ["stdout", "stderr", "both"] },
          "lines": { "type": "number" }
        },
        "required": ["id"]
      },
      "output_schema": {
        "type": "object",
        "properties": {
          "stdout": { "type": "array", "items": { "type": "string" } },
          "stderr": { "type": "array", "items": { "type": "string" } }
        }
      }
    },
    {
      "name": "follow",
      "description": "Follow process output in real-time",
      "input_schema": {
        "type": "object",
        "properties": {
          "id": { "type": "string" },
          "stream": { "type": "string", "enum": ["stdout", "stderr", "both"] }
        },
        "required": ["id"]
      },
      "output_schema": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "data": { "type": "string" },
            "stream": { "type": "string" },
            "exit_code": { "type": "number" }
          }
        }
      },
      "streaming": true
    }
  ]
}
```

## Conclusion

The process management system provides a robust foundation for running and managing long-running processes without requiring a daemon. By integrating this system with the prism architecture, Ultraviolet can offer powerful process management capabilities while maintaining the composability and flexibility of the prism model.

Key benefits of this approach include:

1. **Persistence**: Processes survive CLI termination
2. **Isolation**: Process groups ensure clean termination
3. **Observability**: Comprehensive logging and status tracking
4. **Composability**: Integration with the prism architecture
5. **Efficiency**: Minimal overhead for process management
