# Typed Photons and Adapters

## Overview

This document describes the typed photon system and adapter pattern in Ultraviolet, which enables strongly-typed message passing between prisms and automatic type conversion through adapter prisms.

## Core Concepts

### Typed Photons

Beyond the basic value/record/stream photons, the system supports domain-specific photon types that carry semantic meaning:

```rust
enum UVBeam {
    // Base photons
    ValuePhoton {
        id: Uuid,
        value: Value,
    },
    RecordPhoton {
        id: Uuid,
        key_values: HashMap<String, Value>,
    },
    StreamPhoton {
        id: Uuid,
        value: Value,
        is_last: bool,
    },
    
    // Domain-specific photons
    WebRequestPhoton {
        id: Uuid,
        method: String,      // GET, POST, etc.
        url: String,
        headers: Option<HashMap<String, String>>,
        body: Option<Value>,
    },
    CommandPhoton {
        id: Uuid,
        command: String,
        args: Vec<String>,
        env: Option<HashMap<String, String>>,
        working_dir: Option<String>,
    },
    FileOperationPhoton {
        id: Uuid,
        operation: String,   // read, write, list, etc.
        path: String,
        content: Option<String>,
        options: Option<Value>,
    },
}
```

### Adapter Prisms

Adapter prisms transform photons from one type to another, enabling composition of prisms with incompatible photon types:

```json
{
  "name": "record-to-web",
  "description": "Converts record photons to web request photons",
  "wavelengths": [
    {
      "frequency": "transform",
      "input": {
        "type": "record",
        "key_values": [
          { "key": "method", "value": "string" },
          { "key": "url", "value": "string" },
          { "key": "body", "value": "object", "optional": true }
        ]
      },
      "output": {
        "type": "web_request"
      }
    }
  ]
}
```

## Pipeline Composition

Prisms can be composed into pipelines, with adapters automatically inserted where needed:

```json
{
  "name": "api-data-processor",
  "pipeline": [
    {
      "prism": "db:query",
      "frequency": "query",
      "args": { "query": "SELECT url, auth_token FROM api_configs" }
      // Outputs RecordPhoton
    },
    {
      "prism": "transform:record-to-web",
      "frequency": "transform"
      // Transforms RecordPhoton to WebRequestPhoton
    },
    {
      "prism": "http:client"
      // Handles WebRequestPhoton, outputs RecordPhoton
    }
  ]
}
```

## CLI Integration

The typed photon system enables powerful CLI composition through Unix-style pipes:

```bash
# Chain commands with automatic type conversion
$ uv db:query "SELECT url FROM endpoints" | \
  uv transform:to-http | \
  uv http:get | \
  uv jq ".data"

# Filter and transform records
$ uv aws:s3-list | \
  uv filter:field "size>1MB" | \
  uv sort:field "modified" --desc
```

The system automatically:
1. Detects photon type mismatches in the pipeline
2. Locates appropriate adapter prisms
3. Inserts adapters to enable data flow

## Universal Adapter

A configurable universal adapter can handle common transformation cases:

```json
{
  "prism": "adapter:generic",
  "config": {
    "input_type": "record",
    "output_type": "web_request",
    "mappings": {
      "method": "$.http_method",
      "url": "$.endpoint",
      "headers.Authorization": "Bearer $.auth_token"
    }
  }
}
```

## Benefits

1. **Type Safety**: Domain-specific photons ensure correct data structures
2. **Composability**: Prisms can be connected through adapters
3. **Specialization**: Prisms focus on core functionality
4. **Discoverability**: System can suggest compatible prisms and adapters
5. **CLI Integration**: Natural Unix-style composition with pipes

## Implementation Notes

### 1. Photon Type Registry

```rust
pub struct PhotonTypeRegistry {
    types: HashMap<String, PhotonTypeInfo>,
}

impl PhotonTypeRegistry {
    pub fn register_type(&mut self, name: &str, info: PhotonTypeInfo);
    pub fn find_adapter(&self, from: &str, to: &str) -> Option<AdapterInfo>;
}
```

### 2. Adapter Resolution

```rust
pub struct AdapterResolver {
    registry: Arc<PhotonTypeRegistry>,
}

impl AdapterResolver {
    pub async fn resolve_pipeline(&self, pipeline: &[PrismConfig]) -> Vec<PrismConfig>;
    pub async fn find_conversion_path(&self, from: &str, to: &str) -> Option<Vec<AdapterConfig>>;
}
```

### 3. CLI Integration

```rust
pub struct PipelineBuilder {
    resolver: Arc<AdapterResolver>,
}

impl PipelineBuilder {
    pub async fn build_from_cli(&self, args: &[String]) -> Pipeline;
    pub async fn suggest_completions(&self, partial: &str) -> Vec<Suggestion>;
}
```

## Future Extensions

1. **Adapter Optimization**: Minimize number of transformations in pipeline
2. **Custom Type Definitions**: Allow users to define new photon types
3. **Validation Rules**: Type-specific validation for photons
4. **Streaming Adapters**: Handle stream-to-stream transformations
5. **Bidirectional Adapters**: Support two-way conversions

## Example: Building a Data Pipeline

```rust
// Query a database
let query = uv::prism("db:query")
    .arg("SELECT url, method FROM endpoints");

// Transform records to web requests
let transform = uv::prism("transform:record-to-web");

// Make HTTP requests
let http = uv::prism("http:client");

// Build and execute pipeline
let pipeline = uv::pipeline()
    .add(query)
    .add(transform)  // Automatically inserted if needed
    .add(http)
    .build();

pipeline.execute().await?;
```

This creates a powerful system where prisms can be freely composed while maintaining type safety and semantic meaning.
