# Spectrum Format Specification

## Overview

The spectrum format defines a prism's capabilities, including its available frequencies (methods) and their input/output formats. This specification aims to simplify output handling by defining a set of standard photon types that all prisms must adhere to.

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

```json
{
    "name": "aws-curl",
    "version": "1.0.0",
    "namespace": "aws",
    "description": "Curl-based HTTP requests with Amazon internal auth",
    "tags": ["aws", "http"],
    "dependencies": ["aws:auth"],
    "wavelengths": [
        {
            "frequency": "curl.get",
            "description": "Make a GET request",
            "input": {
                "type": "web_request",
                "method": "GET",
                "properties": {
                    "url": { "type": "string" },
                    "headers": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    }
                },
                "required": ["url"]
            },
            "output": {
                "type": "record",
                "key_values": [
                    {
                        "key": "status",
                        "value": "number"
                    },
                    {
                        "key": "body",
                        "value": "string"
                    }
                ]
            }
        }
    ]
}
```

## Adapter System

Prisms can be connected through adapter prisms that transform between photon types:

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

1. **Type Safety**: Domain-specific photons ensure correct data structures
2. **Composability**: Prisms can be connected through adapters
3. **Specialization**: Prisms focus on core functionality
4. **Discoverability**: System can suggest compatible prisms and adapters
5. **CLI Integration**: Natural Unix-style composition with pipes

## Implementation Notes

1. The core library should provide:
   - Standard photon type definitions
   - Schema validation
   - Default renderers for each type
   - Helper functions for emitting standard photons

2. Prisms should:
   - Declare output types in their spectrum
   - Use standard photon types where possible
   - Document any special handling requirements

3. Consumers should:
   - Check photon types before processing
   - Use appropriate renderers for each type
   - Handle unknown types gracefully

See [Typed Photons and Adapters](typed-photons.md) for more details on the typed photon system.
