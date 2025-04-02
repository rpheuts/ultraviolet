# Prism API Design

## Overview

This document outlines the API design for prisms in the Ultraviolet system, focusing on the ergonomic interfaces for pulse communication and refraction handling.

## Core Components

### UVLink

The `UVLink` provides a bidirectional communication channel between system components, with high-level methods for sending and receiving pulse components.

```rust
pub struct UVLink {
    transport: Arc<dyn Transport>,
    // Other fields as needed
}

impl UVLink {
    /// Create a new link with the given transport
    pub fn new(transport: Arc<dyn Transport>) -> Self { ... }
    
    /// Create a pair of connected links
    pub fn create_pair(factory: &dyn TransportFactory) -> (UVLink, UVLink) { ... }
    
    // Basic pulse operations
    
    /// Send a wavefront to initiate a request
    pub async fn send_wavefront(&self, id: Uuid, frequency: &str, input: Value) -> Result<()> { ... }
    
    /// Emit a photon as a response
    pub async fn emit_photon(&self, id: Uuid, data: Value) -> Result<()> { ... }
    
    /// Emit a trap to signal completion or error
    pub async fn emit_trap(&self, id: Uuid, error: Option<UVError>) -> Result<()> { ... }
    
    /// Receive the next pulse component (wavefront, photon, or trap)
    pub async fn receive(&self) -> Result<Option<(Uuid, UVPulse)>> { ... }
    
    // High-level convenience methods
    
    /// Absorb all photons and deserialize into the expected type
    pub async fn absorb<T>(&self) -> Result<T> 
    where 
        T: DeserializeOwned 
    {
        let mut data = Vec::new();
        
        // Collect all photons until we get a trap
        while let Some((id, pulse)) = self.receive().await? {
            match pulse {
                UVPulse::Photon(photon) => {
                    data.push(photon.data);
                },
                UVPulse::Trap(trap) => {
                    // If there's an error, return it
                    if let Some(error) = trap.error {
                        return Err(error);
                    }
                    // Otherwise, we're done collecting
                    break;
                },
                _ => continue, // Ignore other pulse types
            }
        }
        
        // If we have multiple photons, combine them into an array
        let result = if data.len() > 1 {
            serde_json::to_value(data)?
        } else if data.len() == 1 {
            data.into_iter().next().unwrap()
        } else {
            // No data received
            return Err(UVError::Other("No data received".to_string()));
        };
        
        // Deserialize into the expected type
        let typed_result = serde_json::from_value(result)?;
        Ok(typed_result)
    }

    /// Reflect data back as photon(s) and a success trap
    pub async fn reflect<T>(&self, id: Uuid, data: T) -> Result<()> 
    where 
        T: Serialize 
    {
        let value = serde_json::to_value(data)?;
        
        // If it's an array, send multiple photons
        if let Value::Array(items) = value {
            for item in items {
                self.emit_photon(id, item).await?;
            }
        } else {
            // Otherwise, send a single photon
            self.emit_photon(id, value).await?;
        }
        
        // Send a success trap
        self.emit_trap(id, None).await?;
        Ok(())
    }
}
```

### PrismMultiplexer

The `PrismMultiplexer` manages the loading, initialization, and connection of prisms. It provides a centralized way to handle prism lifecycle and communication.

```rust
pub struct PrismMultiplexer {
    registry: Arc<PrismRegistry>,
    transport_factory: Arc<dyn TransportFactory>,
    spectrum_loader: Arc<dyn SpectrumLoader>,
}

impl PrismMultiplexer {
    /// Create a new PrismMultiplexer
    pub fn new(
        registry: Arc<PrismRegistry>,
        transport_factory: Arc<dyn TransportFactory>,
        spectrum_loader: Arc<dyn SpectrumLoader>,
    ) -> Self {
        Self {
            registry,
            transport_factory,
            spectrum_loader,
        }
    }
    
    /// Load a spectrum for a prism
    pub async fn load_spectrum(&self, prism_id: &str) -> Result<UVSpectrum> {
        self.spectrum_loader.load(prism_id).await
    }
    
    /// Lazy-load a prism if not already loaded
    pub async fn load_prism(&self, prism_id: &str) -> Result<Box<dyn UVPrism>> {
        // Get the prism factory from the registry
        let factory = self.registry.get(prism_id)
            .ok_or_else(|| UVError::PrismNotFound(prism_id.to_string()))?;
            
        // Create a new prism instance
        let mut prism = factory.create();
        
        // Load the spectrum for the prism
        let spectrum = self.load_spectrum(prism_id).await?;
        
        // Initialize the prism with its spectrum
        prism.init(spectrum).await?;
        
        Ok(prism)
    }
    
    /// Apply property mapping according to a schema
    pub fn apply_mapping(&self, schema: &HashMap<String, String>, value: Value) -> Result<Value> {
        let mapper = PropertyMapper::new(schema.clone());
        mapper.apply_transpose(&value)
    }
    
    /// Connect to a prism and get a link for communication
    pub async fn connect_to_prism(&self, prism_id: &str) -> Result<UVLink> {
        // Load the prism
        let prism = self.load_prism(prism_id).await?;
        
        // Create a pair of connected links
        let (system_link, prism_link) = UVLink::create_pair(&*self.transport_factory);
        
        // Load the spectrum for the prism
        let spectrum = self.load_spectrum(prism_id).await?;
        
        // Create a PrismCore to manage the prism
        let mut core = PrismCore::new(prism, spectrum, Arc::clone(self));
        
        // Establish the link with the core
        core.establish_link(prism_link).await?;
        
        // Spawn a task to run the core's attenuate method
        tokio::spawn(async move {
            core.attenuate().await;
        });
        
        // Return the system link for communication with the prism
        Ok(system_link)
    }
    
    /// Call a refraction on a target prism
    pub async fn refract(&self, refraction: &Refraction, payload: Value) -> Result<UVLink> {
        // Parse the target into namespace and name
        let (namespace, prism_name) = refraction.parse_target()?;
        let target_id = format!("{}:{}", namespace, prism_name);
        
        // Apply transpose mapping to the payload
        let mapped_payload = self.apply_mapping(&refraction.transpose, payload)?;
        
        // Connect to the target prism
        let link = self.connect_to_prism(&target_id).await?;
        
        // Send the wavefront to the target
        let request_id = Uuid::new_v4();
        link.send_wavefront(request_id, &refraction.frequency, mapped_payload).await?;
        
        // Return the link for receiving responses
        Ok(link)
    }
}
```

### Spectrum Loader Interface

The `SpectrumLoader` interface provides a way to load spectrum definitions for prisms from various sources.

```rust
#[async_trait]
pub trait SpectrumLoader: Send + Sync {
    /// Load a spectrum for a prism
    async fn load(&self, prism_id: &str) -> Result<UVSpectrum>;
}

/// File-based spectrum loader
pub struct FileSpectrumLoader {
    base_path: PathBuf,
}

#[async_trait]
impl SpectrumLoader for FileSpectrumLoader {
    async fn load(&self, prism_id: &str) -> Result<UVSpectrum> {
        // Parse the prism ID to get namespace and name
        let parts: Vec<&str> = prism_id.split(':').collect();
        if parts.len() != 2 {
            return Err(UVError::InvalidPrismId(prism_id.to_string()));
        }
        
        let namespace = parts[0];
        let name = parts[1];
        
        // Construct the path to the spectrum file
        let path = self.base_path
            .join(namespace)
            .join(name)
            .with_extension("spectrum.json");
        
        // Read and parse the spectrum file
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| UVError::SpectrumLoadError(e.to_string()))?;
            
        let spectrum: UVSpectrum = serde_json::from_str(&content)
            .map_err(|e| UVError::SpectrumParseError(e.to_string()))?;
            
        Ok(spectrum)
    }
}
```

### PrismCore

The `PrismCore` provides core functionality for prisms, including refraction handling and the main processing loop. It uses the `PrismMultiplexer` for managing prism connections.

```rust
pub struct PrismCore {
    spectrum: UVSpectrum,
    multiplexer: Arc<PrismMultiplexer>,
    refraction_cache: HashMap<String, RefractedPrism>,
    prism: Box<dyn UVPrism>,
    link: Option<UVLink>,
}

impl PrismCore {
    /// Create a new PrismCore
    pub fn new(prism: Box<dyn UVPrism>, spectrum: UVSpectrum, multiplexer: Arc<PrismMultiplexer>) -> Self {
        Self {
            spectrum,
            multiplexer,
            refraction_cache: HashMap::new(),
            prism,
            link: None,
        }
    }
    
    /// Establish a link with the prism
    pub async fn establish_link(&mut self, link: UVLink) -> Result<()> {
        // Store the link
        self.link = Some(link.clone());
        
        // Call the prism's on_link_established hook
        self.prism.on_link_established(&link).await?;
        
        Ok(())
    }
    
    /// Run the main processing loop
    pub async fn attenuate(&self) {
        let link = match &self.link {
            Some(link) => link,
            None => return,
        };
        
        // Main processing loop
        while let Ok(Some((id, pulse))) = link.receive().await {
            match &pulse {
                UVPulse::Extinguish => {
                    // Let the prism handle the extinguish pulse first
                    let _ = self.prism.handle_pulse(id, &pulse, link).await;
                    
                    // Call the shutdown hook
                    if let Err(e) = self.prism.on_shutdown().await {
                        log::error!("Error during prism shutdown: {}", e);
                    }
                    
                    // Then extinguish all refractions
                    if let Err(e) = self.extinguish_refractions().await {
                        log::error!("Error extinguishing refractions: {}", e);
                    }
                    
                    break; // Exit the loop
                },
                _ => {
                    // For all other pulses, delegate to the prism
                    match self.prism.handle_pulse(id, &pulse, link).await {
                        Ok(true) => {
                            // Pulse was handled by the prism
                            continue;
                        },
                        Ok(false) => {
                            // Prism chose to ignore this pulse
                            // We could add default handling here if needed
                            continue;
                        },
                        Err(e) => {
                            // Error handling the pulse
                            if let UVPulse::Wavefront(_) = pulse {
                                // For wavefronts, send an error trap
                                let _ = link.emit_trap(id, Some(e)).await;
                            } else {
                                // For other pulses, just log the error
                                log::error!("Error handling pulse: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Call a refraction and get a link for responses
    pub async fn refract(&self, name: &str, payload: Value) -> Result<UVLink> {
        // Look up the refraction in the spectrum
        let refraction = self.spectrum.find_refraction(name)
            .ok_or_else(|| UVError::RefractionError(format!("Refraction not found: {}", name)))?;
        
        // Check if we already have a connection to this prism
        let target_id = refraction.target.clone();
        if let Some(cached) = self.refraction_cache.get(&target_id) {
            // Apply transpose mapping
            let mapped_payload = self.multiplexer.apply_mapping(&refraction.transpose, payload)?;
            
            // Send the wavefront
            let request_id = Uuid::new_v4();
            cached.link.send_wavefront(request_id, &refraction.frequency, mapped_payload).await?;
            
            // Return a clone of the link
            return Ok(cached.link.clone());
        }
        
        // If not cached, use the multiplexer to handle the refraction
        let link = self.multiplexer.refract(refraction, payload).await?;
        
        // Cache the connection for future use
        // (In a real implementation, we'd need to handle thread safety here)
        // self.refraction_cache.insert(target_id, RefractedPrism { link: link.clone() });
        
        Ok(link)
    }
    
    /// Extinguish all refractions
    pub async fn extinguish_refractions(&self) -> Result<()> {
        for (_, refracted) in &self.refraction_cache {
            refracted.link.send_pulse(UVPulse::Extinguish).await?;
        }
        Ok(())
    }
}

/// A cached refraction connection
struct RefractedPrism {
    link: UVLink,
}
```

### UVPulse Types

The `UVPulse` enum defines the types of messages that can be sent over a link:

```rust
pub enum UVPulse {
    Wavefront(Wavefront),  // Initial request with frequency and input
    Photon(Photon),        // Response data carrier
    Trap(Trap),            // Completion signal for a specific request
    Extinguish,            // Signal to terminate the entire link/prism
}
```

The `Extinguish` pulse type is used to signal that a prism should terminate, cleaning up its resources and propagating the termination to any prisms it has refracted to.

### UVPrism Trait

The `UVPrism` trait defines the interface for all prisms in the system. It uses a handler-based approach where the prism implements handlers for specific pulse types.

```rust
#[async_trait]
pub trait UVPrism: Send + Sync {
    /// Initialize the prism with its spectrum
    async fn init(&mut self, spectrum: UVSpectrum) -> Result<()>;
    
    /// Called when a link is established with the prism
    /// This is a setup hook, not for processing
    async fn on_link_established(&mut self, link: &UVLink) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }
    
    /// Handle any pulse received on the link
    /// Returns true if the pulse was handled, false if it should be ignored
    async fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool>;
    
    /// Called when the prism is about to be terminated
    /// This is a cleanup hook, not for processing
    async fn on_shutdown(&self) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }
}
```

This handler-based approach provides several benefits:
1. **Flexibility**: Prisms can handle any current or future pulse type
2. **Selective Handling**: Prisms can choose which pulses to handle and which to ignore
3. **Future-Proof**: We can add new pulse types without changing the UVPrism trait
4. **Clean Separation**: Infrastructure concerns are handled by PrismCore, while business logic is in the prism

## Usage Examples

### Simple Prism Implementation

```rust
struct EchoPrism {
    core: PrismCore,
}

#[async_trait]
impl UVPrism for EchoPrism {
    async fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        // Create a PrismCore with the spectrum and a reference to the multiplexer
        self.core = PrismCore::new(spectrum, Arc::clone(&GLOBAL_MULTIPLEXER));
        Ok(())
    }
    
    async fn on_link_established(&mut self, link: &UVLink) -> Result<()> {
        // Any setup that needs to happen when the link is established
        log::info!("Echo prism link established");
        Ok(())
    }
    
    async fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        match pulse {
            UVPulse::Wavefront(wavefront) => {
                match wavefront.frequency.as_str() {
                    "echo" => {
                        // Simply reflect the input back
                        link.reflect(id, wavefront.input.clone()).await?;
                        Ok(true) // Pulse handled
                    },
                    _ => {
                        // Unknown frequency
                        link.emit_trap(id, Some(UVError::MethodNotFound(
                            wavefront.frequency.clone()
                        ))).await?;
                        Ok(true) // Pulse handled
                    }
                }
            },
            UVPulse::Extinguish => {
                // Clean up any resources
                log::info!("Echo prism shutting down");
                Ok(true) // Pulse handled
            },
            _ => {
                // Ignore other pulse types
                Ok(false) // Pulse not handled
            }
        }
    }
    
    async fn on_shutdown(&self) -> Result<()> {
        // Any cleanup that needs to happen when the prism is shutting down
        log::info!("Echo prism shutdown complete");
        Ok(())
    }
}
```

### Using Refractions

```rust
struct BurnerPrism {
    core: PrismCore,
}

#[async_trait]
impl UVPrism for BurnerPrism {
    async fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        // Create a PrismCore with the spectrum and a reference to the multiplexer
        self.core = PrismCore::new(spectrum, Arc::clone(&GLOBAL_MULTIPLEXER));
        Ok(())
    }
    
    async fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        match pulse {
            UVPulse::Wavefront(wavefront) => {
                match wavefront.frequency.as_str() {
                    "list_accounts" => {
                        // Handle the list_accounts frequency
                        self.handle_list_accounts(id, wavefront.input.clone(), link).await?;
                        Ok(true) // Pulse handled
                    },
                    "complex_operation" => {
                        // Handle the complex_operation frequency
                        self.handle_complex_operation(id, wavefront.input.clone(), link).await?;
                        Ok(true) // Pulse handled
                    },
                    _ => {
                        // Unknown frequency
                        link.emit_trap(id, Some(UVError::MethodNotFound(
                            wavefront.frequency.clone()
                        ))).await?;
                        Ok(true) // Pulse handled
                    }
                }
            },
            _ => {
                // Ignore other pulse types
                Ok(false) // Pulse not handled
            }
        }
    }
}

impl BurnerPrism {
    async fn handle_list_accounts(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Call the http.get refraction and absorb the result
        let accounts: AccountList = self.core.refract("http.get", json!({
            "url": "https://api.aws.com/accounts"
        })).await?
          .absorb::<AccountList>().await?;
        
        // Process the accounts if needed
        let processed_accounts = self.process_accounts(accounts)?;
        
        // Reflect the result back
        link.reflect(id, processed_accounts).await?;
        
        Ok(())
    }
    
    // For more complex cases where you need more control:
    async fn handle_complex_operation(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let http_link = self.core.refract("http.get", json!({
            "url": input["url"].as_str().unwrap_or("https://api.aws.com/accounts")
        })).await?;
        
        // Process responses manually if needed
        while let Some((_, pulse)) = http_link.receive().await? {
            match pulse {
                UVPulse::Photon(photon) => {
                    // Custom processing
                    let processed = self.custom_process(photon.data)?;
                    link.emit_photon(id, processed).await?;
                },
                UVPulse::Trap(trap) => {
                    // Forward the trap or handle errors
                    link.emit_trap(id, trap.error).await?;
                    break;
                },
                _ => continue,
            }
        }
        
        Ok(())
    }
    
    fn process_accounts(&self, accounts: AccountList) -> Result<Value> {
        // Process the accounts
        // ...
        Ok(json!({"processed": true, "count": accounts.len()}))
    }
    
    fn custom_process(&self, data: Value) -> Result<Value> {
        // Custom processing
        // ...
        Ok(json!({"processed": true, "data": data}))
    }
}
```

### Chaining Operations

```rust
// Concise chaining of operations
let result: AccountInfo = self.core.refract("http.get", json!({
    "url": "https://api.aws.com/accounts"
}))
.await?
.absorb::<AccountInfo>()
.await?;
```

## Transport Abstraction

The transport layer provides a low-level abstraction for sending and receiving raw data.

```rust
/// Transport abstraction for sending and receiving raw data
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send raw data over the transport
    async fn send(&self, data: Vec<u8>) -> Result<()>;
    
    /// Receive raw data from the transport
    async fn receive(&self) -> Result<Option<Vec<u8>>>;
    
    /// Close the transport
    async fn close(&self) -> Result<()>;
}

/// Factory for creating transport pairs
pub trait TransportFactory {
    /// Create a pair of connected transports
    fn create_pair(&self) -> (Box<dyn Transport>, Box<dyn Transport>);
}
```

## Benefits

### API Benefits

1. **Type Safety**: Generic methods like `absorb` and `reflect` provide type safety through Rust's type system.
2. **Ergonomic API**: High-level methods make common operations concise and readable.
3. **Flexibility**: Developers can choose between high-level methods for simple cases and low-level methods for complex cases.
4. **Consistent Interface**: All links have the same capabilities, simplifying the mental model.
5. **Clean Chaining**: Operations can be chained for concise, readable code.
6. **Transport Agnosticism**: The transport layer can be swapped out without affecting the higher-level API.

### Handler-Based Approach Benefits

1. **Separation of Concerns**: Infrastructure concerns (processing loop, error handling) are handled by PrismCore, while business logic is in the prism.
2. **Future-Proof**: New pulse types can be added without changing the UVPrism trait.
3. **Selective Handling**: Prisms can choose which pulses to handle and which to ignore.
4. **Lifecycle Hooks**: The `on_link_established` and `on_shutdown` hooks provide clear points for setup and cleanup.
5. **Centralized Resource Management**: PrismCore manages refraction caching and cleanup, reducing duplication.
6. **Simplified Error Handling**: Errors are handled consistently by the PrismCore.
7. **Extinguish Propagation**: Termination signals are automatically propagated to all refractions.

## Implementation Notes

1. The `absorb` method handles both single and multiple photons, automatically combining multiple photons into an array if needed.
2. The `reflect` method handles both single values and arrays, automatically sending multiple photons for arrays.
3. Error handling is consistent throughout, with errors from traps being propagated appropriately.
4. The transport abstraction allows for different transport mechanisms (in-memory, network, etc.) without affecting the higher-level API.

## Prism Lifecycle

The prism lifecycle consists of several distinct phases:

1. **Creation**: A prism instance is created by a factory.
2. **Initialization**: The prism is initialized with its spectrum via the `init` method.
3. **Link Establishment**: A communication link is established with the prism via the `establish_link` method.
4. **Attenuation**: The prism processes wavefronts and emits photons in its `attenuate` method.
5. **Completion**: The prism completes its work and returns from the `attenuate` method.

### Asynchronous Execution Model

The handler-based approach enables an asynchronous execution model:

```
┌─────────────────────┐                  ┌─────────────────────┐                  ┌─────────────────────┐
│                     │                  │                     │                  │                     │
│   PrismMultiplexer  │                  │     PrismCore       │                  │       Prism         │
│                     │                  │                     │                  │                     │
└─────────┬───────────┘                  └─────────┬───────────┘                  └─────────┬───────────┘
          │                                        │                                        │
          │ 1. Create prism                        │                                        │
          │                                        │                                        │
          │ 2. Initialize with spectrum            │                                        │
          │                                        │                                        │
          │ 3. Create PrismCore                    │                                        │
          ├───────────────────────────────────────►│                                        │
          │                                        │                                        │
          │ 4. Create link pair                    │                                        │
          │                                        │                                        │
          │ 5. Establish link                      │                                        │
          │                                        ├───────────────────────────────────────►│
          │                                        │                                        │
          │                                        │ 6. Call on_link_established            │
          │                                        │◄───────────────────────────────────────┤
          │                                        │                                        │
          │ 7. Spawn task for attenuate            │                                        │
          │                                        │                                        │
          │                                        │                                        │
┌─────────▼───────────┐                  ┌─────────▼───────────┐                  ┌─────────────────────┐
│                     │                  │                     │                  │                     │
│    System Task      │                  │    Core Task        │                  │       Prism         │
│                     │                  │                     │                  │                     │
└─────────┬───────────┘                  └─────────┬───────────┘                  └─────────┬───────────┘
          │                                        │                                        │
          │ 8. Send wavefront                      │                                        │
          ├───────────────────────────────────────►│                                        │
          │                                        │                                        │
          │                                        │ 9. Delegate to handle_pulse            │
          │                                        ├───────────────────────────────────────►│
          │                                        │                                        │
          │                                        │                                        │ 10. Process wavefront
          │                                        │                                        │
          │                                        │◄───────────────────────────────────────┤
          │                                        │ 11. Return result                      │
          │                                        │                                        │
          │ 12. Receive photons                    │                                        │
          │◄───────────────────────────────────────┤                                        │
          │                                        │                                        │
          │ 13. Receive trap                       │                                        │
          │◄───────────────────────────────────────┤                                        │
          │                                        │                                        │
```

This model allows:

1. **Concurrent Execution**: Multiple prisms can run concurrently in separate tasks.
2. **Resource Isolation**: Failures in one prism don't affect others.
3. **Clean Separation**: Setup (link establishment) is separate from execution (attenuation).
4. **Error Handling**: Errors during setup are returned directly, while errors during execution are sent through the link.

### Synchronous Execution Model

For simpler cases, a synchronous execution model is also possible:

```rust
// Create and initialize the prism
let mut prism = factory.create();
prism.init(spectrum.clone()).await?;

// Create a pair of connected links
let (system_link, prism_link) = UVLink::create_pair(&transport_factory);

// Create a PrismCore to manage the prism
let mut core = PrismCore::new(prism, spectrum, Arc::clone(&multiplexer));

// Establish the link with the core
core.establish_link(prism_link).await?;

// Send a wavefront
system_link.send_wavefront(id, "echo", json!({"message": "Hello"})).await?;

// Run the core synchronously (this will block until the prism is done)
core.attenuate().await;

// Process any responses that were sent before the prism completed
while let Ok(Some((id, pulse))) = system_link.receive().await {
    // Process the pulse
}
```

This approach is useful for:
1. **Simple Prisms**: Prisms that don't need to run for a long time.
2. **Testing**: Easier to test prisms in a synchronous manner.
3. **Resource Constraints**: When spawning many tasks is not desirable.
