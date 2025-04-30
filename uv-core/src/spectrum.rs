//! Spectrum format types for the Ultraviolet system.
//!
//! The spectrum format defines a prism's capabilities, including its available
//! frequencies (methods), their input/output formats, and dependencies on other
//! prisms through refractions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::fs;
use jsonschema::JSONSchema;

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
    pub wavelengths: Vec<UVWavelength>,
    
    /// Dependencies on other prisms
    #[serde(default)]
    pub refractions: Vec<Refraction>,
}

/// Definition of a method/function that a prism can handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UVWavelength {
    /// Name of the frequency (method)
    pub frequency: String,
    
    /// Description of what the frequency does
    pub description: String,
    
    /// Input schema definition
    pub input: UVSchemaDefinition,
    
    /// Output schema definition
    pub output: UVSchemaDefinition,
}

/// Schema definition for input/output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UVSchemaDefinition {
    /// The schema definition, typically a JSON Schema
    #[serde(flatten)]
    pub schema: Value,
    
    /// List of required properties
    #[serde(default)]
    pub required: Vec<String>,
}

impl UVSpectrum {    
    /// Find a wavelength by its frequency name
    pub fn find_wavelength(&self, frequency: &str) -> Option<&UVWavelength> {
        self.wavelengths.iter().find(|w| w.frequency == frequency)
    }
    
    /// Get the input schema for a specific frequency
    pub fn get_input_schema(&self, frequency: &str) -> Option<&UVSchemaDefinition> {
        self.find_wavelength(frequency).map(|w| &w.input)
    }
    
    /// Get the output schema for a specific frequency
    pub fn get_output_schema(&self, frequency: &str) -> Option<&UVSchemaDefinition> {
        self.find_wavelength(frequency).map(|w| &w.output)
    }
    
    /// Find a refraction by its name
    pub fn find_refraction(&self, name: &str) -> Option<&Refraction> {
        self.refractions.iter().find(|r| r.name == name)
    }

    pub fn resolve_prism_id(name:&String) -> Result<(String, String), UVError> {
        let install_dir = Self::get_install_dir()?
            .join("prisms");

        if let Some((namespace, prism_name)) = name.split_once(':') {
            if namespace.is_empty() || prism_name.is_empty() {
                return Err(UVError::InvalidInput(format!("Invalid prism ID format: {}", name)));
            }
            return Ok((prism_name.to_string(), namespace.to_string()));
        }

        let prisms = Self::find_prisms(&install_dir, &name.to_string())?;

        match prisms.len() {
            0 => Err(UVError::InvalidInput(format!("No prism found for: {}", name))),
            1 => {
                let mut path = prisms[0].clone();
                path.pop(); // remove file name
                let namespace = path.file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| UVError::InvalidInput("Invalid path structure".to_string()))?;
                Ok((name.to_string(), namespace.to_string()))
            },
            _ => Err(UVError::InvalidInput(format!(
                "Multiple prisms found, please specify namespace for: {}",
                name
            ))),
        }
    }

    pub fn new(prism_id: &str) -> Result<UVSpectrum, UVError> {
        // Parse the prism ID to get namespace and name
        let parts: Vec<&str> = prism_id.split(':').collect();
        if parts.len() != 2 {
            return Err(UVError::InvalidInput(format!("Invalid prism ID format: {}", prism_id)));
        }

        let path = Self::get_install_dir()?
            .join("prisms")
            .join(parts[0])
            .join(parts[1])
            .join("spectrum.json");
            
            if path.exists() {
                return Self::try_load_from_path(&path);
            }
        
        // If we get here, we couldn't find the spectrum file
        Err(UVError::Other(format!("Spectrum file not found for prism: {}", prism_id)))
    }

    pub fn new_from_json(json: String) -> Result<UVSpectrum, UVError> {
        return Self::try_load(&json);
    }

    /// Get the standard installation directory for prisms.
    fn get_install_dir() -> Result<PathBuf, UVError> {
        let home_dir = std::env::var("HOME").map_err(|_| UVError::Other("HOME environment variable not set".to_string()))?;
        let install_dir = std::env::var("UV_INSTALL_DIR").unwrap_or(format!("{}/.uv", home_dir));
        Ok(PathBuf::from(install_dir))
    }

    fn find_prisms(path: &PathBuf, prism: &String) -> Result<Vec<PathBuf>, UVError> {
        let mut result: Vec<PathBuf> = Vec::new();

        for dir in fs::read_dir(path)? {
            let sub_path = dir?.path();
            if sub_path.is_dir() {
                if sub_path.file_name().unwrap().eq(OsStr::new(&prism)) {
                    result.push(sub_path);
                } else {
                    result.append(&mut Self::find_prisms(&sub_path, &prism)?);
                }
            }
        }

        Ok(result)
    }
    
    /// Try to load a spectrum from a specific path.
    fn try_load_from_path(path: &Path) -> Result<UVSpectrum, UVError> {
        // Read and parse the spectrum file
        let content = fs::read_to_string(path)
            .map_err(|e| UVError::Other(format!("Failed to read spectrum file {}: {}", path.display(), e)))?;
            
        let spectrum: UVSpectrum = serde_json::from_str(&content)?;
        
        Ok(spectrum)
    }

    /// Try to load a spectrum from a specific path.
    fn try_load(content: &String) -> Result<UVSpectrum, UVError> {
        let spectrum: UVSpectrum = serde_json::from_str(content)?;
        
        Ok(spectrum)
    }
}

impl UVSchemaDefinition {   
    pub fn validate(&self, value: &Value) -> Result<(), UVError> {
        JSONSchema::compile(&self.schema)
            .map_err(|e| UVError::InvalidInput(format!("Input validation error: {}", e)))?
            .validate(value)
            .map_err(|_e| UVError::InvalidInput(format!("Input validation error")))?;
        Ok(())
    }
}