# Process Management System Design

## Overview

The Process Management System enables Ultraviolet to run and manage long-running processes without requiring a daemon. This document describes the architecture and implementation details of this system, which is integrated with the thread-based prism architecture.

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
pub fn spawn(&mut self) -> Result<()> {
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
pub fn is_running(&self) -> Result<bool> {
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
pub fn terminate(&mut self) -> Result<()> {
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
pub fn remove(&mut self) -> Result<()> {
    // If process is running, stop it first
    if self.is_running()? {
        self.terminate()?;
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

Process output is captured and managed using synchronous file operations:

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
pub fn read_lines(&self, stream: Stream, lines: usize) -> Result<Vec<String>> {
    let path = self.output_dir.join(stream.filename());
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = std::fs::File::open(&path)?;
    let reader = std::io::BufReader::new(file);
    let mut lines_vec = Vec::new();
    
    for line in reader.lines() {
        lines_vec.push(line?);
    }

    // Keep only the last N lines
    if lines_vec.len() > lines {
        lines_vec = lines_vec.split_off(lines_vec.len() - lines);
    }

    Ok(lines_vec)
}

pub fn follow(&self, stream: Stream, writer: &mut dyn Write) -> Result<()> {
    let path = self.output_dir.join(stream.filename());
    if !path.exists() {
        return Ok(());
    }

    let file = std::fs::File::open(&path)?;
    let reader = std::io::BufReader::new(file);
    
    for line in reader.lines() {
        writeln!(writer, "{}", line?)?;
        writer.flush()?;
    }

    Ok(())
}
```

## Integration with Thread-Based Prism Architecture

The process management system integrates with the thread-based prism architecture:

### Process Prism

A specialized prism that manages external processes:

```rust
struct ProcessPrism {
    core: PrismCore,
    processes: HashMap<String, Process>,
}

impl UVPrism for ProcessPrism {
    fn init_spectrum(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.core = PrismCore::new(spectrum, Arc::clone(&GLOBAL_MULTIPLEXER));
        Ok(())
    }
    
    fn init_multiplexer(&mut self, multiplexer: Arc<PrismMultiplexer>) -> Result<()> {
        self.core.set_multiplexer(multiplexer);
        Ok(())
    }
    
    fn spectrum(&self) -> &UVSpectrum {
        self.core.spectrum()
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        match pulse {
            UVPulse::Wavefront(wavefront) => {
                match wavefront.frequency.as_str() {
                    "start" => {
                        // Extract process config from wavefront
                        let config = self.extract_process_config(&wavefront.input)?;
                        
                        // Create and spawn process
                        let mut process = Process::new(config)?;
                        process.spawn()?;
                        
                        // Store process
                        self.processes.insert(process.id().to_string(), process);
                        
                        // Return process ID
                        link.emit_photon(id, json!({ "id": process.id() }))?;
                        link.emit_trap(id, None)?;
                        
                        Ok(true)
                    },
                    "stop" => {
                        // Extract process ID from wavefront
                        let process_id = wavefront.input.get("id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| UVError::InvalidInput("Missing process ID".into()))?;
                        
                        // Find and stop process
                        if let Some(mut process) = self.processes.get_mut(process_id) {
                            process.terminate()?;
                            link.emit_photon(id, json!({ "success": true }))?;
                        } else {
                            link.emit_trap(id, Some(UVError::NotFound("Process not found".into())))?;
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
                    if process.is_running()? {
                        let _ = process.terminate();
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

Process output can be streamed using a dedicated thread:

```rust
fn stream_output(&self, process_id: &str, link: &UVLink, request_id: Uuid) -> Result<()> {
    if let Some(process) = self.processes.get(process_id) {
        let output_dir = process.config().output_dir.clone();
        let link = link.clone(); // Links should implement Clone for sharing across threads
        let request_id = request_id;
        
        // Spawn a dedicated thread for output streaming
        std::thread::spawn(move || {
            let mut stdout_file = match std::fs::File::open(output_dir.join("stdout.log")) {
                Ok(file) => file,
                Err(e) => {
                    let _ = link.emit_trap(
                        request_id, 
                        Some(UVError::IoError(format!("Failed to open stdout file: {}", e)))
                    );
                    return;
                }
            };
                
            let mut buf = [0; 4096];
            
            loop {
                match stdout_file.read(&mut buf) {
                    Ok(0) => {
                        // End of file, wait and try again
                        std::thread::sleep(Duration::from_millis(100));
                    },
                    Ok(n) => {
                        // Send data as photon
                        let data = String::from_utf8_lossy(&buf[0..n]).to_string();
                        if let Err(e) = link.emit_photon(request_id, json!({ "data": data })) {
                            let _ = link.emit_trap(request_id, Some(UVError::IoError(format!("Failed to emit photon: {}", e))));
                            break;
                        }
                    },
                    Err(e) => {
                        // Send error and break
                        let _ = link.emit_trap(request_id, Some(UVError::IoError(format!("Failed to read stdout: {}", e))));
                        break;
                    }
                }
                
                // Check if process is still running
                let process_running = match process_running_check(pid) {
                    Ok(running) => running,
                    Err(_) => false,
                };
                
                if !process_running {
                    // Process has terminated, send final event
                    let exit_code = match read_exit_code_file(&exit_file) {
                        Ok(code) => code,
                        Err(_) => 0,
                    };
                    
                    // Send final exit code
                    let _ = link.emit_photon(request_id, json!({ 
                        "exit_code": exit_code 
                    }));
                    let _ = link.emit_trap(request_id, None);
                    break;
                }
            }
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
  "wavelengths": [
    {
      "frequency": "start",
      "description": "Start a new process",
      "input": {
        "command": "string",
        "args": ["string"],
        "env": {
          "additionalProperties": "string"
        },
        "working_dir": "string"
      },
      "output": {
        "id": "string"
      }
    },
    {
      "frequency": "stop",
      "description": "Stop a running process",
      "input": {
        "id": "string",
        "force": "boolean"
      },
      "output": {
        "success": "boolean"
      }
    },
    {
      "frequency": "list",
      "description": "List processes",
      "input": {
        "status": "string",
        "filter": "string"
      },
      "output": {
        "processes": [
          {
            "id": "string",
            "pid": "number",
            "status": "string",
            "command": "string",
            "started_at": "string",
            "exit_code": "number"
          }
        ]
      }
    },
    {
      "frequency": "logs",
      "description": "Get process logs",
      "input": {
        "id": "string",
        "stream": "string",
        "lines": "number"
      },
      "output": {
        "stdout": ["string"],
        "stderr": ["string"]
      }
    },
    {
      "frequency": "follow",
      "description": "Follow process output in real-time",
      "input": {
        "id": "string",
        "stream": "string"
      },
      "output": [
        {
          "data": "string",
          "stream": "string",
          "exit_code": "number"
        }
      ]
    }
  ]
}
```

## Thread Safety Considerations

The process management system is designed to be thread-safe:

1. **State Management**:
   - File system operations use proper locking to prevent race conditions
   - Process state modifications are synchronized through mutexes
   - State file updates are atomic where possible

2. **Process Operations**:
   - Process start/stop operations are atomic and thread-safe
   - PID tracking handles race conditions for terminated processes
   - Signal handling is safe across threads

3. **Output Streaming**:
   - Each output stream runs in its own dedicated thread
   - Thread handles are stored to prevent early termination
   - File operations use proper locking to prevent conflicts

## Conclusion

The process management system provides a robust foundation for running and managing long-running processes without requiring a daemon. By integrating this system with the thread-based prism architecture, Ultraviolet can offer powerful process management capabilities while maintaining the composability and flexibility of the prism model.

Key benefits of this approach include:

1. **Persistence**: Processes survive CLI termination
2. **Isolation**: Process groups ensure clean termination
3. **Observability**: Comprehensive logging and status tracking
4. **Composability**: Integration with the prism architecture
5. **Efficiency**: Minimal overhead for process management
6. **Thread Safety**: Safe operation in a thread-based architecture
