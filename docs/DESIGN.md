# Ultraviolet Design Document: Prism-Based Distributed System Architecture

## 1. Overview

Ultraviolet is a modular, composable, distributed system that enables:

1. Executing small, focused functional units (prisms) that can be composed
2. Multiple interface methods (CLI, web, programmatic API)
3. Local and distributed computation across a network
4. AI orchestration and composition of functionality
5. Long-running or scheduled operations

This document outlines the architectural foundation to achieve these goals incrementally, allowing immediate utility while supporting long-term vision.

## 2. Core Concepts & Terminology

### Prisms
Discrete functional units with well-defined inputs and outputs. A prism processes an incoming pulse (request) and emits photons (responses).

### Pulses
The communication protocol between components:
- **Wavefront**: Incoming request with frequency (method) and input data
- **Photon**: Response data carrier (one or many based on output schema)
- **Trap**: Completion signal with optional error information
- **Extinguish**: Signal to terminate a prism and its refractions

### Spectrum
Metadata describing a prism's capabilities, including:
- Available frequencies (methods)
- Input/output schemas
- Dependencies
- Authentication requirements

### Link
Communication channel between system components that handles serialization, transport, and routing.

### Waveguide
Component responsible for locating, loading, and communicating with prisms.

## 3. Architecture Layers

Ultraviolet follows a layered architecture to support progressive enhancement:

```
┌───────────────────────────────────────────────────────────┐
│                      Interface Layer                      │
│        CLI         Web UI       API        LLM Agent      │
└───────────────────────────┬───────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────┐
│                    Orchestration Layer                    │
│     Composition    │   Scheduling   │     Discovery       │
└───────────────────────────┬───────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────┐
│                     Transport Layer                       │
│      Local         │     Remote     │    Streaming        │
└───────────────────────────┬───────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────┐
│                      Execution Layer                      │
│   Prism Loading  │  Context Mgmt  │ Result Processing     │
└───────────────────────────┬───────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────┐
│                       Prism Layer                         │
│   Local Prisms   │  Remote Prisms  │  Dynamic Loading     │
└───────────────────────────────────────────────────────────┘
```

## 4. Evolution Roadmap

### Phase 1: Foundation (Current Work)
- Async execution framework
- Local prism loading and execution
- Basic CLI interface
- JSON-based pulse protocol

### Phase 2: Enhanced Protocol
- Correlation IDs for request-response tracking
- Structured pulse types
- Enhanced error handling
- Basic composition support

### Phase 3: Distribution
- Remote prism execution
- Network transport layer
- Registry for prism discovery
- Authentication & authorization

### Phase 4: Orchestration
- Prism workflows & pipelines
- Long-running operation support
- Scheduling capabilities
- Event-driven execution

### Phase 5: AI Integration
- LLM-based orchestration
- Capability description format
- Natural language interfaces
- Self-evolving compositions

## 5. Key Technical Decisions

### 5.1 Pulse Protocol Evolution

The pulse protocol has evolved to a simplified, schema-driven model that provides a foundation for all communication between prisms. This enables a clean, consistent system where prisms communicate using well-defined message types.

See [Pulse Protocol](specs/pulse-protocol.md) for detailed documentation of the protocol.

Key concepts:

1. **Pulse Components**
   - Wavefront: Initial request with frequency and input data
   - Photon: Response data carrier (one or many based on output schema)
   - Trap: Completion signal with optional error information
   - Extinguish: Signal to terminate a prism and its refractions

2. **Schema-Driven Communication**
   - Input/output formats defined in the spectrum
   - Single vs. multiple photons determined by output schema
   - No need for specialized photon types

3. **Refraction System**
   - Prisms declare dependencies on other prisms through refractions
   - Refractions define explicit property mapping between prisms
   - Lazy loading ensures prisms are only loaded when needed
   - Clear error propagation through trap photons

4. **CLI Integration**
   ```bash
   # Chain commands with automatic property mapping
   $ uv db:query "SELECT url FROM endpoints" | \
     uv http:get | \
     uv jq ".data"
   ```

This approach enables:
- Simplified protocol with fewer message types
- Self-contained prisms with explicit dependencies
- Schema-driven data flow
- Flexible composition at multiple levels
- Unix-style CLI integration
- Lazy loading for better performance

### 5.2 Transport Abstraction

Create a transport-agnostic layer that works across:
- In-process function calls
- Pipes/stdin+stdout (for CLI)
- TCP/WebSockets (for network)
- Message queue systems (for scaling)

```rust
#[async_trait]
trait Transport: Send + Sync {
    async fn send(&self, pulse: UVPulse) -> Result<()>;
    async fn receive(&self) -> Result<Option<UVPulse>>;
    async fn close(&self) -> Result<()>;
}

// Implementations
struct LocalPipeTransport { /* ... */ }
struct TcpTransport { /* ... */ }
struct WebSocketTransport { /* ... */ }
```

### 5.3 Prism Execution Models

Support different execution models based on use case:

1. **Same-task**: Simplest model, direct execution (current implementation)
2. **Task-per-prism**: Each prism runs in its own tokio task
3. **Process-per-prism**: Isolation through separate processes
4. **Remote-prism**: Execution on remote node

### 5.4 Link Interface

The Link provides a bidirectional communication channel between system components:

```rust
pub struct UVLink {
    transport: Arc<dyn UVTransport>,
    // Internal channels, etc.
}

impl UVLink {
    // Create a pair of connected links
    pub fn create_pair() -> (UVLink, UVLink);
    
    // Receiving methods
    pub async fn receive(&self) -> Result<Option<(Uuid, UVPulse)>>;
    
    // Sending methods
    pub async fn send_wavefront(&self, id: Uuid, frequency: &str, input: Value) -> Result<()>;
    pub async fn emit_photon(&self, id: Uuid, data: Value) -> Result<()>;
    pub async fn emit_trap(&self, id: Uuid, error: Option<UVError>) -> Result<()>;
    pub async fn send_pulse(&self, pulse: UVPulse) -> Result<()>; // For Extinguish and other special pulses
}
```

This interface enables:
- Self-contained prisms that manage their own processing
- Bidirectional communication through a single channel
- Support for various transport mechanisms
- Correlation between requests and responses via request IDs
- Schema-driven data flow based on spectrum definitions

### 5.5 Example Prism Implementation

Here's how a simple echo prism might be implemented using the handler-based approach:

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
                    "echo_stream" => {
                        // For array outputs, we can send multiple photons
                        if let Value::Array(items) = &wavefront.input {
                            for item in items {
                                link.emit_photon(id, item.clone()).await?;
                            }
                            // Signal successful completion
                            link.emit_trap(id, None).await?;
                        } else {
                            // Just echo back the single input
                            link.reflect(id, wavefront.input.clone()).await?;
                        }
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

This implementation demonstrates:
1. A handler-based prism that focuses on business logic
2. Infrastructure concerns handled by PrismCore
3. Lifecycle hooks for setup and cleanup
4. Selective handling of pulse types
5. Clean error handling through the Result type

### 5.6 Refraction System

Refractions provide a mechanism for prisms to declare dependencies on other prisms and define how data flows between them:

```rust
// PrismCore with refraction support
pub struct PrismCore {
    spectrum: UVSpectrum,
    multiplexer: Arc<PrismMultiplexer>,
}

impl PrismCore {
    // Call a refraction and get a link for responses
    pub async fn refract(&self, name: &str, payload: Value) -> Result<UVLink> {
        // 1. Look up the refraction in the spectrum
        let refraction = self.spectrum.find_refraction(name)
            .ok_or_else(|| UVError::RefractionError(format!("Refraction not found: {}", name)))?;
        
        // 2. Use the multiplexer to handle the refraction
        self.multiplexer.refract(refraction, payload).await
    }
    
    // Process responses from a refraction with reflection mapping
    pub async fn process_refraction_responses(
        &self, 
        refraction_link: &UVLink, 
        refraction: &Refraction,
        output_link: &UVLink,
        output_id: Uuid
    ) -> Result<()> {
        // Process responses until we get a trap
        while let Some((id, pulse)) = refraction_link.receive().await? {
            match pulse {
                UVPulse::Photon(photon) => {
                    // Apply reflection mapping to the data
                    let mapped_data = self.multiplexer.apply_mapping(&refraction.reflection, photon.data)?;
                    
                    // Forward the mapped data to the output link
                    output_link.emit_photon(output_id, mapped_data).await?;
                },
                UVPulse::Trap(trap) => {
                    // Forward any error to the output link
                    output_link.emit_trap(output_id, trap.error).await?;
                    break;
                },
                _ => continue, // Ignore other pulse types
            }
        }
        
        Ok(())
    }
    
    // Convenience method to refract and absorb the result
    pub async fn refract_and_absorb<T>(&self, name: &str, payload: Value) -> Result<T>
    where
        T: for<'de> Deserialize<'de>
    {
        let link = self.refract(name, payload).await?;
        link.absorb::<T>().await
    }
    
    // Extinguish all refractions
    pub async fn extinguish_refractions(&self) -> Result<()> {
        for (_, refracted) in &self.refraction_cache {
            refracted.link.send_pulse(UVPulse::Extinguish).await?;
        }
        Ok(())
    }
}

// PrismMultiplexer for managing prism connections
pub struct PrismMultiplexer {
    registry: Arc<PrismRegistry>,
    transport_factory: Arc<dyn TransportFactory>,
    spectrum_loader: Arc<dyn SpectrumLoader>,
}

impl PrismMultiplexer {
    // Connect to a prism and get a link for communication
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
    
    // Call a refraction on a target prism
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

Example refraction declaration in a spectrum:

```json
{
  "name": "http.get",
  "target": "aws:curl",
  "frequency": "curl.get",
  "transpose": {
    "target.url": "refraction.url"
  },
  "reflection": {
    "refraction.body": "reflection.body"
  }
}
```

Key benefits of the refraction system:

1. **Explicit Dependencies**: Prisms clearly declare their dependencies on other prisms
2. **Lazy Loading**: Target prisms are only loaded when actually needed
3. **Property Mapping**: Data flow between prisms is explicit and transparent
4. **Self-Contained**: Prisms remain self-contained while leveraging other prisms
5. **Error Handling**: Consistent error propagation through trap photons

## 6. Immediate Implementation Plan

For the immediate implementation (focusing on Phase 1-2):

1. **Enhance Pulse Protocol**:
   - Add correlation IDs to track request-response pairs
   - Support partial responses for streaming
   - Structured error types

2. **Update Prism Interface**:
   ```rust
   #[async_trait]
   trait UVPrism: Send + Sync {
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
   
   This handler-based approach ensures prisms:
   - Focus on business logic, with infrastructure concerns handled by PrismCore
   - Have clear lifecycle hooks for setup and cleanup
   - Can selectively handle specific pulse types
   - Can be extended with new pulse types without changing the trait
   - Have consistent error handling through the Result type
   
   See [Prism API Design](specs/prism-api.md) for detailed documentation of the ergonomic API design.

3. **Implement Refraction System**:
   - Add refraction declarations to spectrum format
   - Create refraction registry for spectrum validation
   - Implement property mapping for transpose/reflection
   - Add refract API for prisms to call other prisms
   - Support lazy loading of target prisms

4. **Refine Process Stream**:
   - Keep main loop on primary thread for simplicity
   - Use separate task only for output handling
   - Add proper error propagation
   - Pass correlation IDs

5. **Transport Abstraction**:
   - Create transport trait
   - Implement for stdin/stdout (CLI)
   - Allow pulse processor to work with any transport

## 7. Process Management

Ultraviolet includes a robust process management system that enables long-running processes without requiring a daemon. This system is designed to integrate with the prism architecture while providing persistence, isolation, and observability.

See [Process Management](specs/process-management.md) for detailed documentation of the design and implementation.

Key features of the process management system:

1. **Process Lifecycle Management**
   - Unique process identifiers
   - State persistence across CLI invocations
   - Clean process termination using process groups

2. **Output Capture and Access**
   - Stdout/stderr capture to log files
   - Historical and live access to output
   - Log rotation and retention policies

3. **Prism Integration**
   - Process management through the pulse protocol
   - Output streaming via photon sequences
   - Process lifecycle events as pulses

4. **User Interface**
   - Process listing with filtering
   - Status tracking and monitoring
   - Log access and following

## 8. Principles & Guidelines

1. **Progressive Enhancement**: Each phase builds on the previous, providing immediate value while supporting future vision.

2. **Interface Stability**: Core interfaces should remain stable across versions, with extensions rather than breaking changes.

3. **Transport Agnosticism**: Prism logic should be independent of transport mechanism.

4. **Schema-First Design**: All inputs and outputs should be well-defined with schemas.

5. **Self-Description**: Components should be able to describe their capabilities for discovery and composition.

6. **Explicit Dependencies**: Prisms should clearly declare their dependencies through refractions.

7. **Lazy Loading**: Resources should only be loaded when actually needed to improve performance.

8. **Clear Data Flow**: Property mapping should make data flow between components explicit and transparent.
