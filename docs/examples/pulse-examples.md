# Pulse Protocol Examples

This document provides practical examples of how to use the Pulse protocol components in different scenarios.

## Wavefront and Photon Examples

### Echo Command
```rust
async fn handle_frequency(&self, id: Uuid, frequency: &str, input: &Value) -> Result<()> {
    // Simple echo of input
    self.link.emit_photon(id, input.clone()).await?;
    self.link.emit_trap(id, None).await?;
    Ok(())
}
```

CLI Output:
```bash
$ uv test:echo --message "Hello World"
Hello World
```

### Status Check
```rust
async fn handle_frequency(&self, id: Uuid, frequency: &str, input: &Value) -> Result<()> {
    let status = check_service_status().await?;
    self.link.emit_photon(id, json!({ "status": status })).await?;
    self.link.emit_trap(id, None).await?;
    Ok(())
}
```

CLI Output:
```bash
$ uv service:status
running
```

## Array Output Examples

### List Files
```rust
async fn handle_frequency(&self, id: Uuid, frequency: &str, input: &Value) -> Result<()> {
    let mut files = Vec::new();
    
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        
        files.push(json!({
            "name": entry.file_name().to_string_lossy(),
            "size": metadata.len(),
            "modified": metadata.modified()?.into()
        }));
    }
    
    // Emit the array of files as a single photon
    self.link.emit_photon(id, json!(files)).await?;
    self.link.emit_trap(id, None).await?;
    Ok(())
}
```

CLI Output (rendered as a table):
```bash
$ uv fs:list
NAME          SIZE    MODIFIED
README.md     1.2KB   2025-03-26 10:00
src/          -       2025-03-26 09:45
Cargo.toml    340B    2025-03-26 09:30
```

### Process List
```rust
async fn handle_frequency(&self, id: Uuid, frequency: &str, input: &Value) -> Result<()> {
    let processes = system.get_processes()?;
    let process_data: Vec<Value> = processes.iter().map(|p| {
        json!({
            "pid": p.pid(),
            "name": p.name(),
            "memory": p.memory_usage()
        })
    }).collect();
    
    self.link.emit_photon(id, json!(process_data)).await?;
    self.link.emit_trap(id, None).await?;
    Ok(())
}
```

CLI Output (rendered as a table):
```bash
$ uv system:ps
PID     NAME    MEMORY
1234    bash    2.3MB
5678    nginx   156MB
```

## Streaming Examples

### Log Tail with x-uv-stream
```rust
async fn handle_frequency(&self, id: Uuid, frequency: &str, input: &Value) -> Result<()> {
    let mut file = File::open("/var/log/system.log")?;
    let mut buf = String::new();
    
    while let Ok(n) = file.read_line(&mut buf) {
        if n == 0 { break; }
        // For stream output, we emit individual photons with the same ID
        self.link.emit_photon(id, json!({ "line": buf.trim() })).await?;
        buf.clear();
    }
    
    // Signal completion
    self.link.emit_trap(id, None).await?;
    Ok(())
}
```

In spectrum.json:
```json
{
  "frequency": "tail",
  "output": {
    "type": "object",
    "properties": {
      "line": {"type": "string"}
    },
    "x-uv-stream": "text"
  }
}
```

CLI Output:
```bash
$ uv logs:tail
[2025-03-26 10:00:01] Server started
[2025-03-26 10:00:02] Connection accepted
[2025-03-26 10:00:03] Request processed
```

### Progress Updates
```rust
async fn handle_frequency(&self, id: Uuid, frequency: &str, input: &Value) -> Result<()> {
    for i in 0..100 {
        process_chunk(i).await?;
        // Emit progress updates as individual photons
        self.link.emit_photon(id, json!({
            "percent": i,
            "status": "processing"
        })).await?;
        
        // Sleep to simulate work
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Final update
    self.link.emit_photon(id, json!({
        "percent": 100,
        "status": "complete"
    })).await?;
    
    // Signal completion
    self.link.emit_trap(id, None).await?;
    Ok(())
}
```

In spectrum.json:
```json
{
  "frequency": "process",
  "output": {
    "type": "object",
    "properties": {
      "percent": {"type": "number"},
      "status": {"type": "string"}
    },
    "x-uv-stream": "progress"
  }
}
```

CLI Output:
```bash
$ uv task:process
10% complete
20% complete
...
100% complete
```

## Error Handling with Trap

### Handling Errors Gracefully
```rust
async fn handle_frequency(&self, id: Uuid, frequency: &str, input: &Value) -> Result<()> {
    match process_request(input).await {
        Ok(result) => {
            // Success case
            self.link.emit_photon(id, result).await?;
            self.link.emit_trap(id, None).await?;
        },
        Err(error) => {
            // Error case - emit a trap with the error
            self.link.emit_trap(id, Some(error.into())).await?;
        }
    }
    Ok(())
}
```

CLI Output (on error):
```bash
$ uv api:call
Error: Failed to connect: timeout after 30s
```

## Using Refractions

### Calling Other Prisms
```rust
async fn handle_list(&self, id: Uuid, input: &Value) -> Result<()> {
    // Call the http.get refraction
    let http_link = self.refract("http.get", json!({
        "url": "https://api.example.com/accounts"
    })).await?;
    
    // Create a unique ID for this refraction call
    let refraction_id = Uuid::new_v4();
    
    // Send the request
    http_link.send_wavefront(refraction_id, "get", json!({
        "url": "https://api.example.com/accounts"
    }))?;
    
    // Process responses
    let mut accounts = Vec::new();
    
    loop {
        match http_link.receive()? {
            Some((recv_id, UVPulse::Photon(photon))) if recv_id == refraction_id => {
                // Extract account data from response
                let response_data = photon.data;
                
                // Parse account data from response
                if let Ok(parsed_accounts) = parse_accounts(&response_data) {
                    accounts.extend(parsed_accounts);
                }
            },
            Some((recv_id, UVPulse::Trap(trap))) if recv_id == refraction_id => {
                if let Some(err) = trap.error {
                    // Propagate the error to our caller
                    self.link.emit_trap(id, Some(err)).await?;
                    return Ok(());
                }
                // Success - refraction is complete
                break;
            },
            _ => continue, // Ignore other messages
        }
    }
    
    // Send our consolidated result
    self.link.emit_photon(id, json!(accounts)).await?;
    self.link.emit_trap(id, None).await?;
    Ok(())
}
```

## CLI Composition Examples

### Filtering Data
```bash
# List files and filter by size
$ uv fs:list | uv filter:field "size>1MB"
NAME        SIZE    MODIFIED
large.zip   2.5MB   2025-03-26
data.bin    1.2MB   2025-03-26

# Get processes and sort by memory
$ uv system:ps | uv sort:field "memory" --desc
PID     NAME    MEMORY
5678    nginx   156MB
1234    bash    2.3MB
```

### Processing Streams
```bash
# Tail logs and filter errors
$ uv logs:tail | uv filter:contains "ERROR"
[2025-03-26 10:01:23] ERROR: Connection refused
[2025-03-26 10:02:45] ERROR: Timeout

# Monitor and alert
$ uv metrics:watch | uv alert:threshold "cpu>90"
Alert: CPU usage at 95%
Alert: CPU usage at 92%
```

These examples demonstrate how the Pulse protocol components (Wavefront, Photon, and Trap) are used to implement different communication patterns while maintaining composability through the CLI pipeline.
