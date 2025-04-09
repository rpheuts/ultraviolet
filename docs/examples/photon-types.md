# [DEPRECATED] Photon Types Examples

> **DEPRECATION NOTICE**: This document refers to an older architecture that has been replaced by the Pulse Protocol. Please see [Pulse Protocol Examples](pulse-examples.md) for current examples and [Pulse Protocol](../specs/pulse-protocol.md) for detailed documentation.

The rest of this document is kept for historical reference only.

---

This document provides practical examples of how the three core photon types are used in different scenarios.

## Value Photon Examples

### Echo Command
```rust
async fn handle_frequency(&self, link: &UVAsyncLink, frequency: &str, phase: &Value) -> Result<()> {
    // Simple echo of input
    link.emit_value(phase.clone()).await?;
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
async fn handle_frequency(&self, link: &UVAsyncLink, frequency: &str, phase: &Value) -> Result<()> {
    let status = check_service_status().await?;
    link.emit_value(json!({ "status": status })).await?;
    Ok(())
}
```

CLI Output:
```bash
$ uv service:status
running
```

## Record Photon Examples

### List Files
```rust
async fn handle_frequency(&self, link: &UVAsyncLink, frequency: &str, phase: &Value) -> Result<()> {
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        
        link.emit_record(json!({
            "name": entry.file_name(),
            "size": metadata.len(),
            "modified": metadata.modified()?
        })).await?;
    }
    Ok(())
}
```

CLI Output:
```bash
$ uv fs:list
NAME          SIZE    MODIFIED
README.md     1.2KB   2025-03-26 10:00
src/          -       2025-03-26 09:45
Cargo.toml    340B    2025-03-26 09:30
```

### Process List
```rust
async fn handle_frequency(&self, link: &UVAsyncLink, frequency: &str, phase: &Value) -> Result<()> {
    for process in system.processes() {
        link.emit_record(json!({
            "pid": process.pid(),
            "name": process.name(),
            "memory": process.memory_usage()
        })).await?;
    }
    Ok(())
}
```

CLI Output:
```bash
$ uv system:ps
PID     NAME    MEMORY
1234    bash    2.3MB
5678    nginx   156MB
```

## Stream Photon Examples

### Log Tail
```rust
async fn handle_frequency(&self, link: &UVAsyncLink, frequency: &str, phase: &Value) -> Result<()> {
    let mut file = File::open("/var/log/system.log")?;
    let mut buf = String::new();
    
    while let Ok(n) = file.read_line(&mut buf) {
        if n == 0 { break; }
        link.emit_stream(json!(buf.trim())).await?;
        buf.clear();
    }
    Ok(())
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
async fn handle_frequency(&self, link: &UVAsyncLink, frequency: &str, phase: &Value) -> Result<()> {
    for i in 0..100 {
        process_chunk(i).await?;
        link.emit_stream(json!({
            "percent": i,
            "status": "processing"
        })).await?;
    }
    link.emit_stream(json!({
        "percent": 100,
        "status": "complete"
    })).await?;
    Ok(())
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

## Composition Examples

### Filtering Records
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

These examples demonstrate how the three photon types provide a natural way to handle different kinds of data while maintaining composability through the CLI pipeline.
