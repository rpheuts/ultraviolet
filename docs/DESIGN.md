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
Discrete functional units with well-defined inputs and outputs. A prism processes an incoming beam (request) and emits photons (responses).

### Beams
The communication protocol between components:
- **Wavefront**: Incoming request with frequency (method) and phase (parameters)
- **Photon**: Response data
- **Trap**: Error conditions
- **End**: Completion signal

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
- JSON-based beam protocol

### Phase 2: Enhanced Protocol
- Correlation IDs for request-response tracking
- Structured beam types
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

### 5.1 Beam Protocol Evolution

The beam protocol has evolved to support domain-specific typed photons that carry semantic meaning beyond just data structure. This enables a powerful composition system where prisms can communicate using strongly-typed messages.

See [Typed Photons and Adapters](specs/typed-photons.md) for detailed documentation of the typed photon system.

Key concepts:

1. **Base Photon Types**
   - ValuePhoton: Single values (status codes, computed results)
   - RecordPhoton: Key-value pairs (structured data)
   - StreamPhoton: Continuous data (logs, events)

2. **Domain-Specific Photons**
   - WebRequestPhoton: HTTP operations
   - CommandPhoton: System commands
   - FileOperationPhoton: File system operations
   - More can be added as needed

3. **Adapter System**
   - Prisms can focus on specific photon types
   - Adapters handle type conversion
   - Automatic adapter insertion in pipelines
   - Universal adapter for common transformations

4. **CLI Integration**
   ```bash
   # Chain commands with automatic type conversion
   $ uv db:query "SELECT url FROM endpoints" | \
     uv transform:to-http | \
     uv http:get | \
     uv jq ".data"
   ```

This approach enables:
- Type safety through domain-specific photons
- Natural composition through adapters
- Unix-style CLI integration
- Extensibility through new photon types

### 5.2 Transport Abstraction

Create a transport-agnostic layer that works across:
- In-process function calls
- Pipes/stdin+stdout (for CLI)
- TCP/WebSockets (for network)
- Message queue systems (for scaling)

```rust
#[async_trait]
trait Transport: Send + Sync {
    async fn send(&self, beam: UVBeam) -> Result<()>;
    async fn receive(&self) -> Result<Option<UVBeam>>;
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

## 6. Immediate Implementation Plan

For the immediate implementation (focusing on Phase 1-2):

1. **Enhance Beam Protocol**:
   - Add correlation IDs to track request-response pairs
   - Support partial responses for streaming
   - Structured error types

2. **Update Prism Interface**:
   ```rust
   #[async_trait]
   trait UVAsyncPrism: Send + Sync {
       async fn init(&mut self, spectrum: UVSpectrum) -> Result<()>;
       
       // Handle specific frequency
       async fn handle_frequency(
           &self, 
           link: &UVAsyncLink, 
           request_id: Uuid,
           frequency: &str, 
           phase: &Value
       ) -> Result<()>;
   }
   ```

3. **Refine Process Stream**:
   - Keep main loop on primary thread for simplicity
   - Use separate task only for output handling
   - Add proper error propagation
   - Pass correlation IDs

4. **Transport Abstraction**:
   - Create transport trait
   - Implement for stdin/stdout (CLI)
   - Allow beam processor to work with any transport

## 7. Principles & Guidelines

1. **Progressive Enhancement**: Each phase builds on the previous, providing immediate value while supporting future vision.

2. **Interface Stability**: Core interfaces should remain stable across versions, with extensions rather than breaking changes.

3. **Transport Agnosticism**: Prism logic should be independent of transport mechanism.

4. **Schema-First Design**: All inputs and outputs should be well-defined with schemas.

5. **Self-Description**: Components should be able to describe their capabilities for discovery and composition.
