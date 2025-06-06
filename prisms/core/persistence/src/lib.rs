//! Persistence prism implementation for the Ultraviolet system.
//!
//! This crate provides a basic persistence prism that can store and load
//! UV Pulse photons as JSON files, maintaining the exact photon structure
//! and enabling streaming playback.

pub mod spectrum;

use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use spectrum::{LoadRequest, StoreRequest, StoreResponse, StorageMode};
use uv_core::{
    Photon, Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// Persistence prism for storing and loading UV Pulse photons.
pub struct PersistencePrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
    /// Base directory for storage
    base_dir: PathBuf,
}

impl PersistencePrism {
    /// Create a new persistence prism.
    pub fn new() -> Result<Self> {
        // Create base directory in user's home
        let home_dir = dirs::home_dir()
            .ok_or_else(|| UVError::ExecutionError("Could not determine home directory".into()))?;
        
        let base_dir = home_dir.join(".uv").join("data");
        
        // Create the directory if it doesn't exist
        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)
                .map_err(|e| UVError::ExecutionError(format!("Failed to create data directory: {}", e)))?;
        }

        Ok(Self {
            spectrum: None,
            base_dir,
        })
    }

    /// Validate and resolve a file path relative to the base directory
    fn resolve_path(&self, relative_path: &str) -> Result<PathBuf> {
        // Prevent directory traversal attacks
        if relative_path.contains("..") || relative_path.starts_with('/') {
            return Err(UVError::InvalidInput(
                "Path cannot contain '..' or start with '/'".into()
            ));
        }

        let full_path = self.base_dir.join(relative_path);
        
        // Ensure the resolved path is still within our base directory
        if !full_path.starts_with(&self.base_dir) {
            return Err(UVError::InvalidInput(
                "Path resolves outside of data directory".into()
            ));
        }

        Ok(full_path)
    }

    /// Handle 'store' frequency - collect photons and save to file with content-type awareness
    fn handle_store(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: StoreRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid store request: {}", e)))?;

        // Determine storage mode from content type
        let storage_mode = StorageMode::from_content_type(&request.content_type);

        // Resolve the file path
        let file_path = self.resolve_path(&request.path)?;
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| UVError::ExecutionError(format!("Failed to create directories: {}", e)))?;
        }

        // Collect photons until we get a trap
        let mut photons = Vec::new();
        
        loop {
            match link.receive()? {
                Some((pulse_id, pulse)) if pulse_id == id => {
                    match pulse {
                        UVPulse::Photon(photon) => {
                            photons.push(photon);
                        },
                        UVPulse::Trap(trap) => {
                            if let Some(error) = trap.error {
                                // Forward the error
                                link.emit_trap(id, Some(error))?;
                                return Ok(());
                            }
                            // End of photon stream, proceed to save
                            break;
                        },
                        _ => continue, // Ignore other pulse types
                    }
                },
                Some(_) => continue, // Ignore pulses with different IDs
                None => continue, // No message, keep waiting
            }
        }

        // Process and save based on storage mode
        let (file_data, bytes_written) = match &storage_mode {
            StorageMode::Text { field } => {
                // Extract text from specified field and concatenate
                let mut text_content = String::new();
                for photon in &photons {
                    if let Some(text) = photon.data.get(field).and_then(|v| v.as_str()) {
                        text_content.push_str(text);
                    }
                }
                let bytes = text_content.as_bytes().len();
                (text_content, bytes)
            },
            StorageMode::UvPhoton => {
                // Store complete photon objects as JSON
                let json_data = serde_json::to_string_pretty(&photons)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to serialize photons: {}", e)))?;
                let bytes = json_data.as_bytes().len();
                (json_data, bytes)
            }
        };

        // Write to file
        fs::write(&file_path, &file_data)
            .map_err(|e| UVError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Create response
        let response = StoreResponse {
            success: true,
            photons_stored: photons.len(),
            bytes_written,
            path: request.path,
            content_type: request.content_type.clone(),
            storage_mode: storage_mode.to_string(),
        };

        // Emit success response
        link.emit_photon(id, serde_json::to_value(response)?)?;
        link.emit_trap(id, None)?;

        Ok(())
    }

    /// Handle 'load' frequency - read file and stream data back with content-type awareness
    fn handle_load(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: LoadRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid load request: {}", e)))?;

        // Resolve the file path
        let file_path = self.resolve_path(&request.path)?;

        // Check if file exists
        if !file_path.exists() {
            let error = UVError::ExecutionError(format!("File not found: {}", request.path));
            link.emit_trap(id, Some(error))?;
            return Ok(());
        }

        // Determine content type (auto-detect or use specified)
        let content_type = request.content_type.unwrap_or_else(|| {
            // Try to auto-detect based on file content
            self.detect_content_type(&file_path).unwrap_or_else(|| "uv/photon".to_string())
        });

        let storage_mode = StorageMode::from_content_type(&content_type);

        match storage_mode {
            StorageMode::Text { field } => {
                // Read as plain text and emit as text photons
                let text_content = fs::read_to_string(&file_path)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to read text file: {}", e)))?;
                
                // For text mode, we need to split back into meaningful chunks
                // For now, let's split by lines for text/line, or send as single chunk for text/token
                let field_name = field.clone(); // Clone once to avoid ownership issues
                match field_name.as_str() {
                    "line" => {
                        // Split by lines and emit each line
                        for line in text_content.lines() {
                            let mut photon_data = serde_json::Map::new();
                            photon_data.insert(field_name.clone(), serde_json::Value::String(line.to_string()));
                            link.emit_photon(id, serde_json::Value::Object(photon_data))?;
                        }
                    },
                    _ => {
                        // Send as single text chunk
                        let mut photon_data = serde_json::Map::new();
                        photon_data.insert(field_name, serde_json::Value::String(text_content));
                        link.emit_photon(id, serde_json::Value::Object(photon_data))?;
                    }
                }
            },
            StorageMode::UvPhoton => {
                // Read and parse as JSON photon array
                let json_data = fs::read_to_string(&file_path)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to read JSON file: {}", e)))?;

                let photons: Vec<Photon> = serde_json::from_str(&json_data)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to parse JSON: {}", e)))?;

                // Stream each photon back out
                for photon in photons {
                    // Re-emit the photon data with the current request ID
                    link.emit_photon(id, photon.data)?;
                }
            }
        }

        // Signal successful completion
        link.emit_trap(id, None)?;

        Ok(())
    }

    /// Detect content type based on file content
    fn detect_content_type(&self, file_path: &std::path::Path) -> Option<String> {
        // Try to read the first few bytes to detect format
        if let Ok(content) = fs::read_to_string(file_path) {
            let trimmed = content.trim_start();
            if trimmed.starts_with('[') && trimmed.contains("\"id\":") && trimmed.contains("\"data\":") {
                // Looks like a photon JSON array
                return Some("uv/photon".to_string());
            }
            // Otherwise assume it's plain text
            return Some("text/line".to_string());
        }
        None
    }
}

impl UVPrism for PersistencePrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "store" => {
                    self.handle_store(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "load" => {
                    self.handle_load(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                _ => {
                    // Unknown frequency
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    return Ok(true);
                }
            }
        }
        
        // Ignore other pulse types
        Ok(false)
    }
}

#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(PersistencePrism::new().unwrap())
}
