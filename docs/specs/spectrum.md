# Spectrum Format Specification

## Overview

The spectrum format defines a prism's capabilities, including its available frequencies (methods), their input/output formats, and dependencies on other prisms through refractions. This specification aims to provide a standard way for prisms to declare their interfaces and dependencies.

### Pulse Protocol

The Pulse protocol is the communication backbone of the Ultraviolet system, consisting of four fundamental components:

1. **Wavefront**: Initial request with frequency and input data
2. **Photon**: Response data carrier with flexible structure
3. **Trap**: Completion signal with optional error information
4. **Extinguish**: Signal to terminate a prism and its refractions

These components enable efficient communication between prisms, with Photons carrying data of any structure as defined by the prism's output schema.

## Schema Definition

The spectrum format uses JSON Schema to define input and output formats:

### Required and Optional Fields

According to JSON Schema standards, properties are optional by default unless explicitly marked as required:

```json
{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "age": {"type": "integer"}
  },
  "required": ["name"]
}
```

In this example, `name` is required while `age` is optional.

### Handling Optional and Nullable Fields

For fields that might be null or missing, JSON Schema supports an array of types approach:

```json
"properties": {
  "accountName": {"type": "string"},
  "awsAccountId": {"type": ["string", "null"]}
}
```

This allows a property to accept either a string or null value, which is useful for fields that might not be available right away (e.g., the account ID being unavailable while an account is still being created).

Remember that in JSON Schema, properties are optional by default unless they're explicitly included in the `required` array.

### CLI Rendering

The system renders output based on the data structure specified in the spectrum's output schema:

1. **Arrays**: Rendered as formatted tables with column headers derived from the first object's keys
2. **Objects**: Currently rendered as pretty-printed JSON (with plans to implement card-based rendering)
3. **Streams**: For stream data (indicated by `x-uv-stream` extension), outputs content progressively

This behavior enables intuitive display of different data types in the CLI.

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
            "awsAccountId": {"type": ["string", "null"]},
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

The CLI integration enables natural Unix-style composition:

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
6. **Schema Validation**: JSON Schema ensures data consistency
7. **CLI Integration**: Natural Unix-style composition with pipes

## Implementation Notes

1. The core library provides:
   - Pulse protocol implementation (Wavefront, Photon, Trap, Extinguish)
   - Refraction resolution and lazy loading
   - Property mapping utilities
   - Schema validation
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

See [Pulse Protocol](pulse-protocol.md) for more details on the communication protocol between prisms.
