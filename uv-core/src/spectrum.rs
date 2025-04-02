//! Spectrum format types for the Ultraviolet system.
//!
//! The spectrum format defines a prism's capabilities, including its available
//! frequencies (methods), their input/output formats, and dependencies on other
//! prisms through refractions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::{refraction::Refraction, UVError};

/// The main spectrum definition for a prism.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UVSpectrum {
    /// Name of the prism
    pub name: String,
    
    /// Version of the prism
    pub version: String,
    
    /// Namespace the prism belongs to
    pub namespace: String,
    
    /// Description of the prism's functionality
    pub description: String,
    
    /// Tags for categorization and discovery
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// Available frequencies (methods) that the prism can handle
    pub wavelengths: Vec<Wavelength>,
    
    /// Dependencies on other prisms
    #[serde(default)]
    pub refractions: Vec<Refraction>,
}

/// Definition of a method/function that a prism can handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wavelength {
    /// Name of the frequency (method)
    pub frequency: String,
    
    /// Description of what the frequency does
    pub description: String,
    
    /// Input schema definition
    pub input: SchemaDefinition,
    
    /// Output schema definition
    pub output: SchemaDefinition,
}

/// Schema definition for input/output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    /// The schema definition, typically a JSON Schema
    #[serde(flatten)]
    pub schema: Value,
    
    /// List of required properties
    #[serde(default)]
    pub required: Vec<String>,
}

impl UVSpectrum {    
    /// Find a wavelength by its frequency name
    pub fn find_wavelength(&self, frequency: &str) -> Option<&Wavelength> {
        self.wavelengths.iter().find(|w| w.frequency == frequency)
    }
    
    /// Find a refraction by its name
    pub fn find_refraction(&self, name: &str) -> Option<&Refraction> {
        self.refractions.iter().find(|r| r.name == name)
    }

    pub async fn new(prism_id: &str) -> Result<UVSpectrum, UVError> {
        // Parse the prism ID to get namespace and name
        let parts: Vec<&str> = prism_id.split(':').collect();
        if parts.len() != 2 {
            return Err(UVError::InvalidInput(format!("Invalid prism ID format: {}", prism_id)));
        }
        
        let namespace = parts[0];
        let name = parts[1];
        
        // If not found, try the standard location
        if let Ok(install_dir) = Self::get_install_dir() {
            let standard_path = install_dir
                .join("prisms")
                .join(namespace)
                .join(name)
                .join("spectrum.json");
            
            if standard_path.exists() {
                return Self::try_load_from_path(&standard_path).await;
            }
        }
        
        // If we get here, we couldn't find the spectrum file
        Err(UVError::Other(format!("Spectrum file not found for prism: {}", prism_id)))
    }

    /// Get the standard installation directory for prisms.
    fn get_install_dir() -> Result<PathBuf, UVError> {
        let home_dir = std::env::var("HOME").map_err(|_| UVError::Other("HOME environment variable not set".to_string()))?;
        let install_dir = std::env::var("UV_INSTALL_DIR").unwrap_or(format!("{}/.uv", home_dir));
        Ok(PathBuf::from(install_dir))
    }
    
    /// Try to load a spectrum from a specific path.
    async fn try_load_from_path(path: &Path) -> Result<UVSpectrum, UVError> {
        // Read and parse the spectrum file
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| UVError::Other(format!("Failed to read spectrum file {}: {}", path.display(), e)))?;
            
        let spectrum: UVSpectrum = serde_json::from_str(&content)?;
        
        Ok(spectrum)
    }
}