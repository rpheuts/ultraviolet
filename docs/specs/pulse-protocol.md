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
    pub async fn resolve_refraction(&self, prism: &str, refraction: &str) -> Result<UVLink>;
    pub async fn load_target_if_needed(&self, target: &str) -> Result<()>;
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

## Future Extensions

1. **Advanced Property Mapping**: Support for transformations, conditionals, and functions
2. **Refraction Caching**: Cache results of frequently used refractions
3. **Validation Rules**: Schema validation for refraction inputs and outputs
4. **Streaming Refractions**: Optimize for streaming data between prisms
5. **Bidirectional Refractions**: Support two-way communication between prisms

## Example: Using Refractions

```rust
impl BurnerPrism {
    async fn handle_list(&self, request_id: Uuid, input: &Value) -> Result<()> {
        // Call the http.get refraction
        let link = self.refract("http.get", json!({
            "url": "https://api.aws.com/accounts"
        })).await?;
        
        // Process responses
        while let Some((id, pulse)) = link.receive().await? {
            match pulse {
                UVPulse::Photon(photon) => {
                    // Process the data
                    let accounts = parse_accounts(photon.data)?;
                    
                    // Emit our own response
                    self.link.emit_photon(request_id, json!(accounts)).await?;
                },
                UVPulse::Trap(trap) => {
                    if let Some(err) = trap.error {
                        // Handle error or propagate
                        self.link.emit_trap(request_id, Some(err)).await?;
                        return Ok(());
                    }
                    // Successful completion
                    break;
                },
                _ => continue, // Ignore other pulse types
            }
        }
        
        // Signal successful completion
        self.link.emit_trap(request_id, None).await?;
        Ok(())
    }
}
```

This creates a powerful system where prisms can be self-contained while still leveraging functionality from other prisms through clearly defined dependencies.
