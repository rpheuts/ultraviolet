# UV Service Implementation

## Overview

The UV Service provides a WebSocket interface for interacting with prisms, serving as the primary gateway for all clients. Using the same [Pulse Protocol](pulse-protocol.md) that prisms use internally, the service provides a consistent interface for location-transparent prism execution across a network of nodes as outlined in the [Distributed Architecture](distributed-architecture.md) specification.

## Architecture

```
                     +--------------------+
                     |                    |
WebSocket Clients -> | WebSocket Server   |
                     |                    |
                     +--------------------+
                              |
                              v
                     +--------------------+
                     |                    |
                     | Pulse Router       |
                     |                    |
                     +--------------------+
                              |
                     +--------------------+
                     |                    |
                     | Prism Executor     |
                     |                    |
                     +--------------------+
                              |
                              v
          +------------------------------------------+
          |                                          |
          v                                          v
+--------------------+                    +--------------------+
|                    |                    |                    |
| Local Multiplexer  |                    | Remote UV Services |
|                    |                    |                    |
+--------------------+                    +--------------------+
          |                                          |
          v                                          v
+--------------------+                    +--------------------+
|                    |                    |                    |
| Local Prisms       |                    | Remote Prisms      |
|                    |                    |                    |
+--------------------+                    +--------------------+
```

This architecture enables:
1. **Unified Protocol** - The same UVPulse protocol from core to clients
2. **Location Transparency** - Seamless execution across local and remote prisms
3. **Streaming By Default** - All operations support streaming responses

## Core Components

### 1. WebSocket Server

- Built with Axum web framework
- Native WebSocket support for all communications 
- Configurable port, TLS, and other server options

### 2. Pulse Router

- Maps incoming WebSocket connections to the Pulse protocol
- Routes Wavefront messages to appropriate prism handlers
- Manages connection lifecycle and authentication
- Translates file uploads to appropriate data formats

### 3. Prism Executor

- Translates API requests to prism invocations
- Manages UVLink connections to prisms
- Handles streaming responses via WebSockets
- Applies UI inference rules to format responses

### 4. Static Content Server

- Serves the web UI (optional)
- Configurable directory for static assets
- Supports SPA routing

## WebSocket Protocol

The UV Service uses the same Pulse Protocol over WebSockets that prisms use internally. This creates a consistent message-passing architecture throughout the entire system.

### Client to Server Messages

Clients send `Wavefront` pulse messages:

```json
{
  "Wavefront": {
    "id": "9f8d7c6b-5432-1098-abcd-765432109876",
    "frequency": "list",
    "input": {
      "limit": 10,
      "offset": 0
    }
  }
}
```

### Server to Client Messages

Servers respond with `Photon` and `Trap` pulse messages:

```json
// Data responses
{
  "Photon": {
    "id": "9f8d7c6b-5432-1098-abcd-765432109876",
    "data": {
      "name": "example1",
      "status": "active"
    }
  }
}

// Completion/error signal
{
  "Trap": {
    "id": "9f8d7c6b-5432-1098-abcd-765432109876",
    "error": null  // null indicates success, otherwise contains error details
  }
}
```

### UI Inference Messages (Optional)

For clients that support UI rendering inference, the server may send additional metadata about how to render the data:

```json
{
  "UIHint": {
    "id": "9f8d7c6b-5432-1098-abcd-765432109876",
    "component": "table",
    "properties": {
      "sortable": true,
      "filterable": true
    }
  }
}
```

This UI hint is optional and clients can ignore it if they implement their own rendering logic.

## Deployment Options

### API-Only Mode

Runs just the API server, ideal for:
- Headless deployments
- Custom front-ends
- Integration with existing web applications

```
$ uv web --api-only --port 3000
```

### Full Web UI Mode

Serves both API and web UI:
- Interactive UI for exploring prisms
- Rich rendering of prism responses
- Built-in file upload and management

```
$ uv web --port 3000 --ui-dir ./web
```

### Custom UI Mode

Serves API with a custom UI:
- Bring your own HTML/JS/CSS
- Full access to the API
- Customized experience

```
$ uv web --port 3000 --ui-dir /path/to/custom/ui
```

## UI Component Rendering

The web server implements the UI inference system described in [UI Rendering and Inference](ui-rendering.md):

1. For standard HTTP requests, it transforms responses into HTML/JSON with inferred UI components
2. For WebSocket connections, it annotates responses with component type information
3. The web UI client renders these components appropriately

## Implementation Plan

### Phase 1: Core WebSocket Service

- Basic WebSocket server setup using Axum
- Pulse protocol implementation over WebSockets
- UVPulse message serialization/deserialization

### Phase 2: Prism Integration

- Connection routing based on prism/frequency
- Local prism execution
- Remote service proxy implementation

### Phase 3: Basic Web UI

- Simple web interface for exploring prisms
- Component rendering system
- Basic styling

### Phase 4: Enhanced Features

- File upload support
- Authentication options
- Advanced UI components

## Security Considerations

- Authentication options (none, basic, token)
- TLS configuration
- CORS settings for API access
- Rate limiting
- Input validation

## Configuration Options

```yaml
# Example configuration
server:
  port: 3000
  host: 0.0.0.0
  tls:
    enabled: false
    cert_file: path/to/cert.pem
    key_file: path/to/key.pem
  
ui:
  enabled: true
  directory: ./web
  spa_mode: true
  
security:
  auth_mode: none  # none, basic, token
  cors:
    allowed_origins: ["*"]
  rate_limit:
    requests_per_minute: 60
```
