# Spectrum Format Specification

## Overview

The spectrum format defines a prism's capabilities, including its available frequencies (methods), their input/output formats, and dependencies on other prisms through refractions. This specification aims to simplify output handling by defining a set of standard photon types and enabling composable, self-contained prisms.

## Photon Types

### Base Photon Types

These are the fundamental photon types that form the foundation of the system:

#### 1. Value Photon
A single value output, useful for simple responses.

```json
{
    "type": "value",
    "value": "string"  // or "number", "boolean", etc.
}
```

Example use cases:
- Command success/failure status
- Single computed result
- Simple text output
- Default CLI behavior: Direct output to stdout

#### 2. Record Photon
Key-value pairs representing structured data.

```json
{
    "type": "record",
    "key_values": [
        {
            "key": "id",
            "value": "string"
        },
        {
            "key": "timestamp",
            "value": "string"
        }
    ]
}
```

Example use cases:
- List operations (ls, ps, etc)
- Query results
- Status information
- Default CLI behavior: Rendered as table with keys as columns

#### 3. Stream Photon
A sequence of values, useful for continuous or partial outputs.

```json
{
    "type": "stream",
    "value": "string"  // type of each stream element
}
```

Example use cases:
- Log streaming
- Real-time updates
- Large dataset processing
- Default CLI behavior: Line-by-line output to stdout

### Domain-Specific Photon Types

Beyond the base types, prisms can use domain-specific photon types for clearer semantics:

#### Web Request Photon
For HTTP operations:

```json
{
    "type": "web_request",
    "method": "string",      // GET, POST, etc.
    "url": "string",
    "headers": {             // optional
        "type": "object",
        "additionalProperties": { "type": "string" }
    },
    "body": {               // optional
        "type": "object"
    }
}
```

#### Command Photon
For system command execution:

```json
{
    "type": "command",
    "command": "string",
    "args": {
        "type": "array",
        "items": { "type": "string" }
    },
    "env": {                // optional
        "type": "object",
        "additionalProperties": { "type": "string" }
    },
    "working_dir": {        // optional
        "type": "string"
    }
}
```

## Example Manifest

Spectrum files follow the JSON Schema format for defining input and output schemas:

```json
{
  "name": "curl",
  "namespace": "aws",
  "version": "0.1.0",
  "description": "Curl-based HTTP requests with Amazon internal auth",
  "wavelengths": [
    {
      "frequency": "get",
      "description": "Make a GET request",
      "input": {
        "type": "object",
        "properties": {
          "url": {"type": "string"},
          "headers": {
            "type": "object",
            "additionalProperties": {"type": "string"}
          }
        },
        "required": ["url"]
      },
      "output": {
        "type": "object",
        "properties": {
          "status": {"type": "number"},
          "body": {"type": "string"}
        },
        "required": ["status", "body"]
      }
    },
    {
      "frequency": "post",
      "description": "Make a POST request",
      "input": {
        "type": "object",
        "properties": {
          "url": {"type": "string"},
          "body": {"type": "string"},
          "method": {"type": "string"},
          "headers": {
            "type": "object",
            "additionalProperties": {"type": "string"}
          }
        },
        "required": ["url"]
      },
      "output": {
        "type": "object",
        "properties": {
          "status": {"type": "number"},
          "body": {"type": "string"}
        },
        "required": ["status", "body"]
      }
    }
  ]
}
```

Here's a more complex example with refractions:

```json
{
  "name": "burner",
  "namespace": "aws",
  "version": "0.1.0",
  "description": "Manage burner AWS accounts",
  "wavelengths": [
    {
      "frequency": "list",
      "description": "List all burner accounts",
      "input": {
        "type": "object",
        "properties": {}
      },
      "output": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "accountName": {"type": "string"},
            "awsAccountId": {"type": "string"},
            "status": {"type": "string"},
            "validTill": {"type": "string"},
            "user": {"type": "string"}
          }
        }
      }
    },
    {
      "frequency": "create",
      "description": "Create a new burner account",
      "input": {
        "type": "object",
        "properties": {
          "name": {"type": "string"}
        },
        "required": ["name"]
      },
      "output": {
        "type": "object",
        "properties": {
          "success": {"type": "boolean"},
          "message": {"type": "string"},
          "details": {"type": "object"}
        },
        "required": ["success", "message"]
      }
    }
  ],
  "refractions": [
    {
      "name": "curl.get",
      "target": "aws:curl",
      "frequency": "get",
      "transpose": {
        "url": "url",
        "headers": "headers?"
      },
      "reflection": {
        "status": "status",
        "body": "body"
      }
    },
    {
      "name": "curl.post",
      "target": "aws:curl",
      "frequency": "post",
      "transpose": {
        "url": "url",
        "body": "body?",
        "method": "method?",
        "headers": "headers?"
      },
      "reflection": {
        "status": "status",
        "body": "body"
      }
    }
  ]
}
```

### Special Schema Extensions

The spectrum format supports some special extensions to JSON Schema:

#### Streaming Output

For streaming responses, you can use the `x-uv-stream` extension:

```json
{
  "frequency": "stream_logs",
  "description": "Stream log entries",
  "input": {
    "type": "object",
    "properties": {
      "source": {"type": "string"}
    }
  },
  "output": {
    "type": "object",
    "properties": {
      "line": {"type": "string"},
      "timestamp": {"type": "string"}
    },
    "x-uv-stream": "text"  // Indicates this is a stream output
  }
}
```

## Refractions

Refractions are a mechanism for prisms to declare dependencies on other prisms and define how data flows between them. They enable prisms to be self-contained while still leveraging functionality from other prisms.

### Refraction Structure

```json
{
    "name": "curl.get",           // Local name for the refraction
    "target": "aws:curl",         // Target prism in namespace:name format
    "frequency": "get",           // Frequency to call on the target prism
    "transpose": {                // Input mapping (local to target properties)
        "url": "url",             // Map local url to target url
        "headers": "headers?"     // Optional property mapping (? indicates optional)
    },
    "reflection": {               // Output mapping (target to local properties)
        "status": "status",       // Map target status to local status
        "body": "body"            // Map target body to local body
    }
}
```

### Key Concepts

1. **Name**: A local identifier for the refraction, used when calling it from within the prism
2. **Target**: The prism that will be called, specified as namespace:name
3. **Frequency**: The specific method to call on the target prism
4. **Transpose**: Maps input properties from the refraction call to the target prism's input
5. **Reflection**: Maps output properties from the target prism's response back to the caller

### Lazy Loading

Refractions support lazy loading of target prisms:

1. When a prism is loaded, its spectrum is parsed and refractions are validated
2. Target prisms are not loaded until the refraction is actually used
3. Once loaded, target prisms are cached for subsequent calls

### Error Handling

Errors in refractions are propagated through trap photons:

1. If a target prism encounters an error, it emits a trap photon
2. The calling prism receives this trap and can handle it appropriately
3. Traps can be wrapped and propagated upward or handled internally

### Usage in Prism Code

Prisms use refractions through a simple API:

```rust
// Call a refraction and get a link for responses
let link = self.refract("http.get", json!({
    "url": "https://api.example.com/data"
})).await?;

// Process responses from the refraction
while let Some((id, photon)) = link.receive_photon().await? {
    // Handle the response
}
```

## External Composition

While refractions handle dependencies within prisms, external composition allows system users to connect multiple prisms together without modifying them.

### Composition Format

```json
{
  "name": "aws-account-report",
  "pipeline": [
    {
      "prism": "aws:burner",
      "frequency": "list"
    },
    {
      "prism": "aws:cost-explorer",
      "frequency": "analyze",
      "mapping": {
        "input.accounts": "previous.output"
      }
    },
    {
      "prism": "format:report",
      "frequency": "generate",
      "mapping": {
        "input.data": "previous.output",
        "input.format": "pdf"
      }
    }
  ]
}
```

### Key Differences from Refractions

1. **Ownership**: Refractions are defined by prism authors, compositions by system users
2. **Scope**: Refractions are internal to a prism, compositions are external
3. **Lifecycle**: Refractions exist for the lifetime of a prism, compositions are standalone entities
4. **Flexibility**: Compositions can connect any compatible prisms, not just those with declared refractions

## CLI Integration

The photon type system enables natural Unix-style composition:

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

## Benefits

1. **Self-Contained Prisms**: Refractions allow prisms to be self-contained while still leveraging other prisms
2. **Explicit Dependencies**: Dependencies between prisms are clearly declared and documented
3. **Lazy Loading**: Target prisms are only loaded when needed, improving performance
4. **Flexible Composition**: Both internal (refractions) and external composition are supported
5. **Clear Data Flow**: Property mapping makes data flow between prisms explicit and transparent
6. **Type Safety**: Base photon types ensure consistent communication patterns
7. **CLI Integration**: Natural Unix-style composition with pipes

## Implementation Notes

1. The core library should provide:
   - Base photon type definitions
   - Refraction resolution and lazy loading
   - Property mapping utilities
   - Helper functions for emitting photons
   - External composition support

2. Prisms should:
   - Declare refractions in their spectrum
   - Use the refract API for calling other prisms
   - Handle trap photons appropriately
   - Document their frequencies and refractions

3. Consumers should:
   - Use external composition for connecting prisms
   - Leverage the CLI for simple pipelines
   - Create custom compositions for complex workflows

See [Typed Photons and Adapters](typed-photons.md) for more details on the photon system.
