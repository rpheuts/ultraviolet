//! Multiplexer for managing prisms in the Ultraviolet system.
//!
//! The PrismMultiplexer manages prism loading, initialization, and communication.
//! It provides a centralized way to handle prism lifecycle and communication.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use serde_json::Value;
use tokio::spawn;
use libloading::{Library, Symbol};

use crate::error::{UVError, Result};
use crate::link::UVLink;
use crate::prism::UVPrism;
use crate::prism_core::PrismCore;
use crate::refraction::{Refraction, PropertyMapper};
use crate::spectrum::UVSpectrum;

/// Type for a function that creates a prism instance.
type CreatePrismFn = fn() -> Box<dyn UVPrism>;

/// Factory for creating prism instances.
struct PrismFactory {
    library: Arc<Library>,
}

impl PrismFactory {
    /// Create a new prism instance.
    fn create(&self) -> Result<Box<dyn UVPrism>> {
        unsafe {
            let create_prism: Symbol<CreatePrismFn> = self.library.get(b"create_prism")?;
            Ok(create_prism())
        }
    }
}

/// Multiplexer for managing prisms.
#[derive(Clone)]
pub struct PrismMultiplexer {
    /// Map of prism IDs to their factories
    factories: Arc<RwLock<HashMap<String, Arc<PrismFactory>>>>,
    
    /// Paths to search for prism libraries (deprecated)
    library_paths: Arc<RwLock<Vec<PathBuf>>>,
}

impl PrismMultiplexer {
    /// Create a new PrismMultiplexer.
    pub fn new() -> Self {
        Self {
            factories: Arc::new(RwLock::new(HashMap::new())),
            library_paths: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add a directory to search for prism libraries.
    /// This method is kept for backward compatibility but is deprecated.
    pub fn add_library_path(&self, path: impl AsRef<Path>) {
        let mut paths = self.library_paths.write().unwrap();
        paths.push(path.as_ref().to_path_buf());
    }
    
    /// Load prisms from all registered library paths.
    /// This method is kept for backward compatibility but is deprecated.
    pub fn load_prisms(&self) -> Result<()> {
        // This method is now a no-op as prisms are loaded on demand
        Ok(())
    }
    
    /// Load a spectrum for a prism.
    pub async fn load_spectrum(&self, prism_id: &str) -> Result<Arc<UVSpectrum>> {        
        // Load the spectrum
        let spectrum = UVSpectrum::new(prism_id).await?;
        let spectrum_arc = Arc::new(spectrum);
        
        Ok(spectrum_arc)
    }
    
    /// Create a new prism instance.
    fn load_prism(&self, prism_id: &str) -> Result<Box<dyn UVPrism>> {
        // Parse the prism ID to get namespace and name
        let parts: Vec<&str> = prism_id.split(':').collect();
        if parts.len() != 2 {
            return Err(UVError::InvalidInput(format!("Invalid prism ID format: {}", prism_id)));
        }
        
        let namespace = parts[0];
        let name = parts[1];
        
        // Check if we already have this prism loaded
        let factories = self.factories.read().unwrap();
        if let Some(factory) = factories.get(prism_id) {
            return factory.create();
        }
        drop(factories); // Release the read lock
        
        // Determine the prism path
        let home_dir = std::env::var("HOME").map_err(|_| UVError::Other("HOME environment variable not set".to_string()))?;
        let install_dir = std::env::var("UV_INSTALL_DIR").unwrap_or(format!("{}/.uv", home_dir));
        
        // Look for the prism library
        let lib_path = format!("{}/prisms/{}/{}/module", install_dir, namespace, name);
        
        // Try different extensions based on platform
        let extensions = if cfg!(target_os = "windows") {
            vec![".dll"]
        } else if cfg!(target_os = "macos") {
            vec![".dylib"]
        } else {
            vec![".so"]
        };
        
        // Try to load the library with each extension
        for ext in extensions {
            let full_path = format!("{}{}", lib_path, ext);
            let path = Path::new(&full_path);
            
            if path.exists() {
                // Load the library
                let lib = unsafe { Library::new(path) }?;
                
                // Verify that it has the create_prism symbol
                unsafe {
                    // Just check that the symbol exists
                    let _: Symbol<CreatePrismFn> = lib.get(b"create_prism")?;
                }
                
                let lib_arc = Arc::new(lib);
                
                // Create a prism factory
                let factory = Arc::new(PrismFactory {
                    library: lib_arc,
                });
                
                // Register the factory
                let mut factories = self.factories.write().unwrap();
                factories.insert(prism_id.to_string(), factory.clone());
                
                // Create and return a new prism instance
                return factory.create();
            }
        }
        
        // If we get here, we couldn't find the prism in the standard location
        // Try the legacy method of looking in the library paths
        let paths = self.library_paths.read().unwrap();
        for path in paths.iter() {
            if path.is_dir() {
                for entry in std::fs::read_dir(path)? {
                    let entry = entry?;
                    let entry_path = entry.path();
                    
                    if entry_path.is_file() {
                        let extension = entry_path.extension().and_then(|ext| ext.to_str());
                        
                        // Check if this is a dynamic library
                        if let Some(ext) = extension {
                            if ext == "so" || ext == "dylib" || ext == "dll" {
                                let file_name = entry_path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .ok_or_else(|| UVError::Other("Invalid library name".to_string()))?;
                                
                                if file_name == name || file_name == prism_id {
                                    // Load the library
                                    let lib = unsafe { Library::new(&entry_path) }?;
                                    
                                    // Verify that it has the create_prism symbol
                                    unsafe {
                                        // Just check that the symbol exists
                                        let _: Symbol<CreatePrismFn> = lib.get(b"create_prism")?;
                                    }
                                    
                                    let lib_arc = Arc::new(lib);
                                    
                                    // Create a prism factory
                                    let factory = Arc::new(PrismFactory {
                                        library: lib_arc,
                                    });
                                    
                                    // Register the factory
                                    let mut factories = self.factories.write().unwrap();
                                    factories.insert(prism_id.to_string(), factory.clone());
                                    
                                    // Create and return a new prism instance
                                    return factory.create();
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // No library found
        Err(UVError::Other(format!("No library found for prism: {}", prism_id)))
    }
    
    /// Connect to a prism and get a link for communication.
    pub async fn establish_link(&self, prism_id: &str) -> Result<UVLink> {
        // Create a new prism instance
        let mut prism = self.load_prism(prism_id)?;
        
        // Load the spectrum for the prism
        let spectrum = self.load_spectrum(prism_id).await?;
        
        // Initialize the prism with its spectrum
        prism.init((*spectrum).clone()).await?;
        
        // Create a pair of connected links
        let (system_link, prism_link) = UVLink::create_link();
        
        // Create a PrismCore to manage the prism
        let mut core = PrismCore::new(prism, Arc::new(self.clone()));
        
        // Establish the link with the core
        core.establish_link(prism_link).await?;
        
        // Spawn a task to run the core's attenuate method
        spawn(async move {
            core.attenuate().await;
        });
        
        // Return the system link for communication with the prism
        Ok(system_link)
    }
    
    /// Call a refraction on a target prism.
    pub async fn refract(&self, refraction: &Refraction, payload: Value) -> Result<UVLink> {
        // Parse the target into namespace and name
        let (namespace, name) = refraction.parse_target()?;
        let target_id = format!("{}:{}", namespace, name);
        
        // Apply transpose mapping to the payload
        let mapper = PropertyMapper::new(refraction.transpose.clone());
        let mapped_payload = mapper.apply_transpose(&payload)?;
        
        // Connect to the target prism
        let link = self.establish_link(&target_id).await?;
        
        // Send the wavefront to the target
        let request_id = Uuid::new_v4();
        link.send_wavefront(request_id, &refraction.frequency, mapped_payload).await?;
        
        // Return the link for receiving responses
        Ok(link)
    }
}
