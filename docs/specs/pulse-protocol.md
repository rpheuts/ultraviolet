# Pulse Protocol

## Overview

This document describes the pulse protocol in Ultraviolet, which enables strongly-typed message passing between prisms. The protocol is designed to be simple, flexible, and schema-driven.

## Core Concepts

### Pulse Components

The pulse protocol consists of three fundamental components:

```rust
enum UVPulse {
    // Initial request
    Wavefront {
        id: Uuid,
        frequency: String,
        input: Value,  // Structured according to the frequency's input schema
    },
    
    // Response data (one or many based on output schema)
    Photon {
        id: Uuid,       // Matches the wavefront id
        data: Value,    // Structured according to the frequency's output schema
    },
    
    // Completion/error signal
    Trap {
        id: Uuid,       // Matches the wavefront id
        error: Option<UVError>,  // None means successful completion
    },
    
    // Termination signal
    Extinguish,  // No parameters needed - signals prism to shut down
}
```

#### Wavefront
The initial request that starts a pulse. It contains:
- A unique identifier for correlation
- The frequency (method) to invoke
- Input data structured according to the frequency's input schema

#### Photon
The response data carrier. It contains:
- A correlation ID matching the wavefront
- Data structured according to the frequency's output schema
- Can be sent multiple times if the output schema defines an array type

#### Trap
The pulse terminator that signals completion. It contains:
- A correlation ID matching the wavefront
- An optional error (None indicates successful completion)

#### Extinguish
The termination signal that instructs a prism to shut down. It contains:
- No parameters - it's a simple signal
- When received, causes the prism to clean up resources and exit its thread
- Should be propagated to any child prisms or refractions

### Refractions

Refractions allow prisms to declare dependencies on other prisms and define how data flows between them:

```rust
struct Refraction {
    name: String,           // Local name for the refraction
    target: String,         // Target prism in namespace:name format
    frequency: String,      // Frequency to call on the target prism
    transpose: HashMap<String, String>,  // Input mapping
    reflection: HashMap<String, String>, // Output mapping
}
```

Example refraction in a spectrum:

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

## Thread-Based Implementation

The pulse protocol is implemented using thread-safe channels:

```rust
pub struct UVLink {
    sender: crossbeam_channel::Sender<UVPulse>,
    receiver: crossbeam_channel::Receiver<UVPulse>,
}

impl UVLink {
    // Create a pair of connected links
    pub fn create_pair() -> (UVLink, UVLink) {
        let (tx1, rx1) = crossbeam_channel::unbounded();
        let (tx2, rx2) = crossbeam_channel::unbounded();
        
        (
            UVLink { sender: tx1, receiver: rx2 },
            UVLink { sender: tx2, receiver: rx1 },
        )
    }
    
    // Send a wavefront pulse
    pub fn send_wavefront(&self, id: Uuid, frequency: &str, input: Value) -> Result<()> {
        self.sender.send(UVPulse::Wavefront {
            id,
            frequency: frequency.to_string(), 
            input
        })?;
        Ok(())
    }
    
    // Receive the next pulse
    pub fn receive(&self) -> Result<Option<(Uuid, UVPulse)>> {
        match self.receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(pulse) => {
                // For Extinguish, use a dummy UUID since it doesn't have an ID
                let id = match &pulse {
                    UVPulse::Extinguish => Uuid::nil(),
                    _ => pulse.id(),
                };
                Ok(Some((id, pulse)))
            },
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => Ok(None),
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                Err(UVError::ConnectionClosed)
            }
        }
    }
    
    // Send an extinguish signal
    pub fn send_extinguish(&self) -> Result<()> {
        self.sender.send(UVPulse::Extinguish)?;
        Ok(())
    }
    
    // Emit a photon pulse
    pub fn emit_photon(&self, id: Uuid, data: Value) -> Result<()> {
        self.sender.send(UVPulse::Photon { id, data })?;
        Ok(())
    }
    
    // Emit a trap pulse
    pub fn emit_trap(&self, id: Uuid, error: Option<UVError>) -> Result<()> {
        self.sender.send(UVPulse::Trap { id, error })?;
        Ok(())
    }
    
    // Absorb all responses and return the final result
    pub fn absorb<T>(&self) -> Result<T>
    where
        T: for<'de> serde::de::DeserializeOwned,
    {
        // Generate a random ID for the correlation
        let request_id = Uuid::new_v4();
        
        // Collect all photons
        let mut data = None;
        
        // Process responses until we get a trap
        loop {
            match self.receive()? {
                Some((id, UVPulse::Photon(photon))) if id == request_id => {
                    data = Some(photon.data);
                },
                Some((id, UVPulse::Trap(trap))) if id == request_id => {
                    if let Some(error) = trap.error {
                        return Err(error);
                    }
                    break;
                },
                Some(_) => continue, // Ignore other messages
                None => {
                    // No message received, continue polling
                    std::thread::sleep(Duration::from_millis(10));
                },
            }
        }
        
        // Deserialize the final data
        if let Some(data) = data {
            return serde_json::from_value(data)
                .map_err(|e| UVError::DeserializationError(format!("{}", e)));
        }
        
        Err(UVError::Other("No data received".into()))
    }
}
```

## External Composition

Prisms can be composed into pipelines using external composition:

```json
{
  "name": "api-data-processor",
  "pipeline": [
    {
      "prism": "db:query",
      "frequency": "query",
      "args": { "query": "SELECT url FROM api_configs" }
    },
    {
      "prism": "aws:curl",
      "frequency": "curl.get",
      "mapping": {
        "input.url": "previous.output.url"
      }
    },
    {
      "prism": "transform:extract",
      "frequency": "json",
      "mapping": {
        "input.data": "previous.output.body"
      }
    }
  ]
}
```

## CLI Integration

The photon system enables powerful CLI composition through Unix-style pipes:

```bash
# Chain commands with automatic type conversion
$ uv db:query "SELECT url FROM endpoints" | \
  uv http:get | \
  uv jq ".data"

# Filter and transform records
$ uv aws:s3-list | \
  uv filter:field "size>1MB" | \
  uv sort:field "modified" --desc
```

The CLI automatically handles:
1. Serializing output from one prism
2. Deserializing input to the next prism
3. Basic property mapping between compatible types

## Property Mapping

Property mapping is a key concept in both refractions and external composition:

```json
{
  "transpose": {
    "target.url": "refraction.url",
    "target.headers.Authorization": "Bearer ${refraction.token}"
  }
}
```

The mapping syntax supports:
1. Direct property mapping (`target.url: refraction.url`)
2. Simple transformations with template strings (`Bearer ${refraction.token}`)
3. Path expressions for nested properties (`headers.Authorization`)

## Benefits

1. **Self-Contained Prisms**: Refractions allow prisms to be self-contained while still leveraging other prisms
2. **Explicit Dependencies**: Dependencies between prisms are clearly declared and documented
3. **Lazy Loading**: Target prisms are only loaded when needed, improving performance
4. **Flexible Composition**: Both internal (refractions) and external composition are supported
5. **Clear Data Flow**: Property mapping makes data flow between prisms explicit and transparent
6. **Error Handling**: Trap photons provide a consistent error handling mechanism
7. **CLI Integration**: Natural Unix-style composition with pipes
8. **Thread Safety**: Communication via thread-safe channels ensures reliable inter-thread messaging
9. **Isolated Execution**: Each prism runs in its own thread for clean isolation and stability

## Implementation Notes

### 1. Refraction Registry

```rust
pub struct RefractionRegistry {
    spectrums: HashMap<String, UVSpectrum>,
}

impl RefractionRegistry {
    pub fn register_spectrum(&mut self, name: &str, spectrum: UVSpectrum);
    pub fn validate_refraction(&self, refraction: &Refraction) -> Result<()>;
    pub fn get_target_spectrum(&self, target: &str) -> Option<&UVSpectrum>;
}
```

### 2. Refraction Resolution

```rust
pub struct RefractionResolver {
    registry: Arc<RefractionRegistry>,
    prism_loader: Arc<PrismLoader>,
}

impl RefractionResolver {
    pub fn resolve_refraction(&self, prism: &str, refraction: &str) -> Result<UVLink>;
    pub fn load_target_if_needed(&self, target: &str) -> Result<()>;
}
```

### 3. Property Mapper

```rust
pub struct PropertyMapper {
    mapping_rules: HashMap<String, String>,
}

impl PropertyMapper {
    pub fn new(mapping_rules: HashMap<String, String>) -> Self;
    pub fn apply_transpose(&self, source: &Value) -> Result<Value>;
    pub fn apply_reflection(&self, target: &Value) -> Result<Value>;
}
```

## Thread-Based Prism Execution

Each prism runs in its own dedicated thread with a simple event loop:

```rust
pub fn run_prism_loop(prism: Box<dyn UVPrism>, link: UVLink) -> Result<()> {
    // Run until we get an Extinguish pulse or the link disconnects
    loop {
        match link.receive() {
            Ok(Some((id, pulse))) => {
                match pulse {
                    UVPulse::Extinguish => {
                        // Clean shutdown
                        break;
                    },
                    _ => {
                        // Handle the pulse
                        if let Err(e) = prism.handle_pulse(id, &pulse, &link) {
                            // Report error
                            let _ = link.emit_trap(id, Some(e));
                        }
                    }
                }
            },
            Ok(None) => {
                // No message received, continue polling
                continue;
            },
            Err(e) => {
                // Connection closed or other error
                log::error!("Prism link error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}
```

## Future Extensions

1. **Advanced Property Mapping**: Support for transformations, conditionals, and functions
2. **Refraction Caching**: Cache results of frequently used refractions
3. **Validation Rules**: Schema validation for refraction inputs and outputs
4. **Streaming Refractions**: Optimize for streaming data between prisms
5. **Bidirectional Refractions**: Support two-way communication between prisms
6. **Thread Pooling**: For systems with many prisms, support thread pooling to limit resource usage

## Example: Using Refractions

```rust
impl BurnerPrism {
    fn handle_list(&self, request_id: Uuid, input: &Value, link: &UVLink) -> Result<()> {
        // Call the http.get refraction
        let http_link = self.refract("http.get", json!({
            "url": "https://api.aws.com/accounts"
        }))?;
        
        // Process responses
        while let Some((id, pulse)) = http_link.receive()? {
            match pulse {
                UVPulse::Photon(photon) => {
                    // Process the data
                    let accounts = parse_accounts(photon.data)?;
                    
                    // Emit our own response
                    link.emit_photon(request_id, json!(accounts))?;
                },
                UVPulse::Trap(trap) => {
                    if let Some(err) = trap.error {
                        // Handle error or propagate
                        link.emit_trap(request_id, Some(err))?;
                        return Ok(());
                    }
                    // Successful completion
                    break;
                },
                _ => continue, // Ignore other pulse types
            }
        }
        
        // Signal successful completion
        link.emit_trap(request_id, None)?;
        Ok(())
    }
}
```

This creates a powerful system where prisms can be self-contained while still leveraging functionality from other prisms through clearly defined dependencies, all running in isolated threads for stability and clean execution.
