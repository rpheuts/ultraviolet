//! Discovery prism implementation for the Ultraviolet system.
//!
//! This prism provides functionality to discover and inspect prisms in the system.

use std::env;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// Discovery prism for finding and inspecting prisms in the system.
pub struct DiscoveryPrism {
    spectrum: Option<UVSpectrum>,
    install_dir: PathBuf,
}

/// Simple struct for prism info without loading the full spectrum
#[derive(Debug, Serialize, Deserialize)]
struct PrismInfo {
    namespace: String,
    name: String,
}

impl DiscoveryPrism {
    /// Create a new discovery prism.
    pub fn new() -> Self {
        let home_dir = env::var("HOME")
            .unwrap_or_else(|_| "/tmp".to_string());
        let install_dir = env::var("UV_INSTALL_DIR")
            .unwrap_or_else(|_| format!("{}/.uv", home_dir));
            
        Self {
            spectrum: None,
            install_dir: PathBuf::from(install_dir),
        }
    }
    
    /// Scan for available prisms without loading spectrum files
    fn scan_available_prisms(&self) -> Result<Vec<PrismInfo>> {
        let mut prisms = Vec::new();
        let prisms_dir = self.install_dir.join("prisms");
        
        if !prisms_dir.exists() {
            return Ok(prisms);
        }
        
        // Iterate through namespace directories
        for namespace_entry in fs::read_dir(&prisms_dir)? {
            let namespace_path = namespace_entry?.path();
            if !namespace_path.is_dir() {
                continue;
            }
            
            let namespace = namespace_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
                
            // Iterate through prism directories in this namespace
            for prism_entry in fs::read_dir(&namespace_path)? {
                let prism_path = prism_entry?.path();
                if !prism_path.is_dir() {
                    continue;
                }
                
                let name = prism_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                    
                // Check if spectrum.json exists (to confirm it's a valid prism)
                let spectrum_path = prism_path.join("spectrum.json");
                if spectrum_path.exists() {
                    prisms.push(PrismInfo {
                        namespace: namespace.clone(),
                        name,
                    });
                }
            }
        }
        
        Ok(prisms)
    }
    
    /// Handle the list frequency
    fn handle_list(&self, id: Uuid, link: &UVLink) -> Result<()> {
        // Scan for available prisms
        let prisms = self.scan_available_prisms()?;
        
        // Emit each prism as a separate photon for streaming support
        for prism in prisms {
            link.emit_photon(id, serde_json::to_value(prism)?)?;
        }
        
        // Signal successful completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle the describe frequency
    fn handle_describe(&self, id: Uuid, prism_id: &str, link: &UVLink) -> Result<()> {
        // Load the spectrum
        let spectrum = UVSpectrum::new(prism_id)?;
        
        // Emit the spectrum as a photon
        link.emit_photon(id, serde_json::to_value(spectrum)?)?;
        
        // Signal successful completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
}

impl UVPrism for DiscoveryPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "list" => {
                    self.handle_list(id, link)?;
                    return Ok(true);
                },
                "describe" => {
                    // Extract prism ID from input
                    let prism_id = wavefront.input.get("prismId")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| UVError::InvalidInput("Missing prismId".to_string()))?;
                    
                    self.handle_describe(id, prism_id, link)?;
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
        
        Ok(false)
    }
}

// Export a function to create a new instance
// This will be used by the dynamic loading system
#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(DiscoveryPrism::new())
}
