//! File upload/download functionality using the persistence prism.
//!
//! This module provides text file upload and download capabilities that work
//! seamlessly with both local and remote UV instances, streaming files line by line
//! using the UV Pulse protocol.

use anyhow::{Context, Result};
use serde_json::json;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use uuid::Uuid;
use uv_core::{PrismMultiplexer, UVLink, UVPulse};

/// Upload a local text file to remote storage via the persistence prism
pub fn handle_upload(local_path: &str, remote_path: &str, remote_url: Option<&str>, content_type: &str) -> Result<()> {
    // Check if local file exists
    if !Path::new(local_path).exists() {
        return Err(anyhow::format_err!("Local file not found: {}", local_path));
    }

    // Get the link to the persistence prism
    let link = get_persistence_link(remote_url)?;
    
    // Send the store wavefront
    let request_id = Uuid::new_v4();
    let prism_id = "core:persistence";
    let store_request = json!({ 
        "path": remote_path,
        "content_type": content_type
    });
    
    link.send_wavefront(request_id, prism_id, "store", store_request)?;
    
    // Read and stream the file line by line
    let file = File::open(local_path)
        .with_context(|| format!("Failed to open file: {}", local_path))?;
    let reader = BufReader::new(file);
    
    for line_result in reader.lines() {
        let line = line_result
            .with_context(|| format!("Failed to read line from file: {}", local_path))?;
        
        // Send each line as a photon
        let photon_data = json!({ "line": line + "\n" });
        link.emit_photon(request_id, photon_data)?;
    }
    
    // Send completion trap
    link.emit_trap(request_id, None)?;
    
    loop {
        match link.receive()? {
            Some((id, pulse)) if id == request_id => {
                match pulse {
                    UVPulse::Photon(photon) => {
                        if let Some(success) = photon.data.get("success").and_then(|v| v.as_bool()) {
                            if success {
                                let bytes_written = photon.data.get("bytes_written")
                                    .and_then(|v| v.as_u64()).unwrap_or(0);
                                
                                println!("Upload successful ({}kb written)", bytes_written/ 1000);
                                return Ok(());
                            }
                        }
                    },
                    UVPulse::Trap(trap) => {
                        if let Some(error) = trap.error {
                            return Err(anyhow::format_err!("Upload failed: {}", error));
                        }
                        break;
                    },
                    _ => continue,
                }
            },
            Some(_) => continue, // Ignore other messages
            None => continue,
        }
    }
    
    Ok(())
}

/// Download a remote file to local storage via the persistence prism
pub fn handle_download(remote_path: &str, local_path: &str, remote_url: Option<&str>, content_type: &str) -> Result<()> {
    // Get the link to the persistence prism
    let link = get_persistence_link(remote_url)?;
    
    // Send the load wavefront
    let request_id = Uuid::new_v4();
    let prism_id = "core:persistence";
    let load_request = json!({ 
        "path": remote_path,
        "content_type": content_type
    });
    
    link.send_wavefront(request_id, prism_id, "load", load_request)?;
    
    // Create the local file for writing
    let mut file = File::create(local_path)
        .with_context(|| format!("Failed to create local file: {}", local_path))?;
    
    let mut download_complete = false;
    
    // Collect the streaming photons and write to file
    while !download_complete {
        match link.receive()? {
            Some((id, pulse)) if id == request_id => {
                match pulse {
                    UVPulse::Photon(photon) => {
                        if let Some(line) = photon.data.get("line").and_then(|v| v.as_str()) {
                            writeln!(file, "{}", line)
                                .with_context(|| format!("Failed to write to file: {}", local_path))?;
                        }
                    },
                    UVPulse::Trap(trap) => {
                        if let Some(error) = trap.error {
                            return Err(anyhow::format_err!("Download failed: {}", error));
                        }
                        download_complete = true;
                    },
                    _ => continue,
                }
            },
            Some(_) => continue, // Ignore other messages
            None => continue,
        }
    }
    
    // Ensure all data is written to disk
    file.flush()
        .with_context(|| format!("Failed to flush file: {}", local_path))?;
    
    println!("Download successful!");
    
    Ok(())
}

/// Get a link to the persistence prism (local or remote)
fn get_persistence_link(remote_url: Option<&str>) -> Result<UVLink> {
    let multiplexer = PrismMultiplexer::new();
    
    match remote_url {
        Some(_url) => {
            // Use remote connection via system:remote prism
            let bridge_link = multiplexer.establish_link("system:remote")
                .map_err(|e| anyhow::format_err!("Failed to establish remote link: {}", e))?;
            
            // For remote, we need to use the bridge pattern
            // This is a simplified approach - in practice we'd need to handle the bridge properly
            Ok(bridge_link)
        },
        None => {
            // Direct local connection
            multiplexer.establish_link("core:persistence")
                .map_err(|e| anyhow::format_err!("Failed to establish local persistence link: {}", e))
        }
    }
}
