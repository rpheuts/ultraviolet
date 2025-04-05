//! Multiplexer for managing prisms in the Ultraviolet system.
//!
//! The PrismMultiplexer manages prism loading, initialization, and communication.
//! It provides a centralized way to handle prism lifecycle and communication.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread;
use serde::de::DeserializeOwned;
use uuid::Uuid;
use serde_json::Value;
use libloading::{Library, Symbol};

use crate::error::{UVError, Result};
use crate::link::UVLink;
use crate::prism::UVPrism;
use crate::prism_core::UVPrismCore;
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
}

impl PrismMultiplexer {
    /// Create a new PrismMultiplexer.
    pub fn new() -> Self {
        Self {
            factories: Arc::new(RwLock::new(HashMap::new()))
        }
    }
    
    /// Load a spectrum for a prism.
    fn load_spectrum(&self, prism_id: &str) -> Result<Arc<UVSpectrum>> {        
        // Load the spectrum
        let spectrum = UVSpectrum::new(prism_id)?;
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
        
        // No library found
        Err(UVError::Other(format!("No library found for prism: {}", prism_id)))
    }
    
    /// Connect to a prism and get a link for communication.
    pub fn establish_link(&self, prism_id: &str) -> Result<UVLink> {
        // Create a pair of connected links
        let (system_link, prism_link) = UVLink::create_link();
        
        // Clone everything needed for the new thread
        let prism_id = prism_id.to_string();

        // Create a new prism instance
        let mut prism = match self.load_prism(&prism_id) {
            Ok(p) => p,
            Err(e) => {
                // Report initialization error
                let _ = prism_link.emit_trap(Uuid::nil(), Some(e));
                return Err(UVError::ExecutionError("Failed to load prism".to_string()));
            }
        };
        
        // Load the spectrum for the prism
        let spectrum = match self.load_spectrum(&prism_id) {
            Ok(s) => s,
            Err(e) => {
                // Report initialization error
                let _ = prism_link.emit_trap(Uuid::nil(), Some(e));
                return Err(UVError::ExecutionError("Failed to load spectrum".to_string()));
            }
        };
        
        // Spawn a thread to run the prism
        thread::spawn(move || {
            // Initialize the prism with its spectrum
            if let Err(e) = prism.init((*spectrum).clone()) {
                // Report initialization error
                let _ = prism_link.emit_trap(Uuid::nil(), Some(e));
                return;
            }
            
            // Establish the link with the prism
            if let Err(e) = prism.link_established(&prism_link) {
                // Report initialization error
                let _ = prism_link.emit_trap(Uuid::nil(), Some(e));
                return;
            }
            
            // Create a PrismCore to manage the prism
            let core = UVPrismCore::new(prism);
            
            // Run the prism's main loop
            if let Err(e) = core.run_loop(prism_link) {
                eprintln!("Error in prism thread: {}", e);
            }
        });
        
        // Return the system link for communication with the prism
        Ok(system_link)
    }
    
    /// Call a refraction on a target prism.
    pub fn refract(&self, refraction: &Refraction, payload: Value) -> Result<UVLink> {
        // Parse the target into namespace and name
        let (namespace, name) = refraction.parse_target()?;
        let target_id = format!("{}:{}", namespace, name);
        
        // Apply transpose mapping to the payload
        let mapper = PropertyMapper::new(refraction.transpose.clone());
        let mapped_payload = mapper.apply_transpose(&payload)?;
        
        // Connect to the target prism
        let link = self.establish_link(&target_id)?;
        
        // Send the wavefront to the target
        let request_id = Uuid::new_v4();
        link.send_wavefront(request_id, &target_id, &refraction.frequency, mapped_payload)?;
        
        // Return the link for receiving responses
        Ok(link)
    }

    pub fn refract_and_absorb<T>(&self, name: &str, spectrum: &UVSpectrum, payload: Value) -> Result<T>
    where
    T: DeserializeOwned {
        // Find the refraction
        let refraction = spectrum
            .find_refraction(name)
            .ok_or_else(|| UVError::RefractionError(format!("{} refraction not found", name)))?;
            
        // Call the refraction
        let link = self.refract(refraction, payload)?;
            
        // Get the response
        link.absorb()
    }
}
