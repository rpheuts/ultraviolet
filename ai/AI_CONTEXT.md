# Ultraviolet (UV) System - AI Context Guide

## Project Overview

Ultraviolet is a distributed, prism-based compute system designed for both human and AI-driven interaction. It enables modular, composable functionality through small functional units called "prisms" that communicate via a structured pulse protocol. The system supports multiple interfaces (CLI, Web, API) and is architected to allow AI agents to orchestrate complex distributed computing tasks.

**Current Status**: Active development, Phase 1-2 implementation with thread-based architecture and basic prism ecosystem.

**Key Value Propositions**:
- Modular, composable distributed computing
- AI-first design enabling natural language orchestration
- Multiple interaction modes (human CLI, web UI, AI agent)
- Schema-driven communication with strong typing
- Thread-isolated execution for stability

## Core Architecture & Concepts

### Prisms
Self-contained functional units that:
- Run in dedicated threads for isolation
- Expose capabilities through "wavelengths" (methods)
- Communicate via the pulse protocol
- Declare dependencies through "refractions"
- Are described by JSON "spectrum" metadata

### Pulse Protocol
Four-component communication system:

```rust
enum UVPulse {
    Wavefront {     // Initial request
        id: Uuid,
        frequency: String,  // Method to invoke
        input: Value,      // Schema-validated input
    },
    Photon {        // Response data (can be multiple)
        id: Uuid,
        data: Value,       // Schema-validated output
    },
    Trap {          // Completion/error signal
        id: Uuid,
        error: Option<UVError>,
    },
    Extinguish,     // Shutdown signal
}
```

**Flow**: Wavefront → Photon(s) → Trap (with optional Extinguish for cleanup)

### Spectrum Format
JSON metadata defining prism capabilities:

```json
{
  "name": "example",
  "namespace": "core",
  "version": "0.1.0",
  "description": "Example prism functionality",
  "wavelengths": [
    {
      "frequency": "process",
      "description": "Process input data",
      "input": { "type": "object", "properties": {...} },
      "output": { "type": "object", "properties": {...} }
    }
  ],
  "refractions": [...]  // Dependencies on other prisms
}
```

### Refractions (Inter-Prism Dependencies)
Declarative system for prism dependencies with property mapping:

```json
{
  "name": "http.get",
  "target": "aws:curl",
  "frequency": "get",
  "transpose": {        // Input mapping
    "url": "url",
    "headers": "headers?"
  },
  "reflection": {       // Output mapping
    "body": "response_body"
  }
}
```

### Thread-Based Execution
- **One thread per prism** for clean isolation
- **Thread-safe channels** for communication via UVLink
- **Synchronous processing** within each prism
- **Lazy loading** of dependent prisms

## System Components

### Core Libraries
- **`uv-core/`**: Fundamental types (UVPulse, UVSpectrum, UVLink, etc.)
- **`uv-service/`**: Service layer for request handling and multiplexing

### Clients
- **CLI** (`clients/cli/`): Interactive modes (chat, command, prism), pipe composition
- **Web UI** (`clients/ultraviolet-web/`): React-based comprehensive chat interface with AI agent support
- **Lambda** (`clients/lambda/`): AWS Lambda integration

### Prism Organization
- **`prisms/ai/`**: AI integration (context, agent, knowledge, mcp)
- **`prisms/aws/`**: AWS services (burner accounts, curl, governance, etc.)
- **`prisms/core/`**: Core system (bedrock, q, persistence)
- **`prisms/system/`**: System utilities (discovery, datetime, timer, remote)

## Web Chat Interface

The Ultraviolet web client (`clients/ultraviolet-web/`) provides a comprehensive chat interface for AI interaction with advanced features:

### Key Features
- **Dual Mode Operation**: Toggle between direct AI chat and AI Agent mode
- **Multiple AI Backends**: Support for AWS Bedrock and Q models with easy switching
- **Context File Management**: Drag-and-drop file upload with context panel
- **Conversation Persistence**: Automatic saving and history management
- **Streaming Responses**: Real-time token streaming for immediate feedback
- **Agent Progress Tracking**: Visual indicators for agent actions and progress
- **Reasoning Display**: Toggle to show/hide AI reasoning processes
- **Action Result Cards**: Structured display of agent action outcomes
- **Retry Functionality**: Smart retry with edit capabilities on failures

### Agent Mode Integration
When in Agent Mode, the web interface:
- Displays structured agent responses with action cards
- Shows real-time progress indicators for long-running operations
- Provides expandable action result details
- Tracks token usage and context window utilization
- Supports conversation branching and history navigation

### Technical Implementation
- **React-based**: Modern component architecture with Material-UI
- **WebSocket Communication**: Real-time bidirectional communication with UV server
- **Service Layer**: Dedicated services for chat and agent interactions
- **Markdown Rendering**: Rich text display with syntax highlighting
- **File Context**: Seamless integration with file-based context

## Remote Prism Execution

The `system:remote` prism enables distributed computing across UV instances:

### system:remote Prism
**Purpose**: Connect to and execute prisms on remote Ultraviolet instances

**Key Wavelength**: `refract`
- Connects to remote UV servers via WebSocket
- Forwards prism execution requests to remote instances
- Handles response streaming and error propagation
- Enables transparent distributed computing

**Input Schema**:
```json
{
  "url": "wss://remote-uv-server.com/ws",
  "prism": "namespace:name",
  "frequency": "method_name",
  "input": { /* method-specific data */ }
}
```

### Use Cases
- **Distributed Processing**: Execute compute-intensive prisms on remote servers
- **Resource Access**: Access prisms with specific hardware or network requirements
- **Load Distribution**: Spread workload across multiple UV instances
- **Specialized Environments**: Execute prisms in environments with specific configurations

### Integration Pattern
```rust
// Example: Remote execution through refraction
{
  "name": "remote.process",
  "target": "system:remote",
  "frequency": "refract",
  "transpose": {
    "url": "remote_server_url",
    "prism": "target_prism",
    "frequency": "target_method",
    "input": "payload"
  },
  "reflection": {
    "result": "response_data"
  }
}
```

## AI Integration (Key Focus)

### ai:context Prism
**Purpose**: Enriches user prompts with UV system context and streams AI responses

**Key Wavelength**: `chat`
- Takes natural language prompts
- Discovers available prisms
- Enriches context with system capabilities
- Streams responses from AI backends

**Refractions**: 
- `discovery.list/describe` - System capability discovery
- `knowledge.search` - Knowledge base integration
- `bedrock.invoke_stream` / `q.invoke_stream` - AI backends

### ai:knowledge Prism
**Purpose**: Persistent knowledge storage and retrieval
- Categorized storage (bootstrap, task, etc.)
- Search capabilities
- Context enrichment for AI agents

### AI Backends
- **core:bedrock**: AWS Bedrock integration with streaming
- **core:q**: Alternative AI backend with streaming

### AI Agent Workflow
1. User provides natural language prompt
2. `ai:context` discovers system capabilities
3. Context enriched with available prisms and knowledge
4. AI backend generates structured responses with actions
5. Actions executed through prism system
6. Results fed back for continued orchestration

## Development Patterns

### Creating New Prisms

1. **Define Spectrum** (`spectrum.json`):
```json
{
  "name": "my-prism",
  "namespace": "custom",
  "wavelengths": [
    {
      "frequency": "process",
      "input": {"type": "object", "properties": {"data": {"type": "string"}}},
      "output": {"type": "object", "properties": {"result": {"type": "string"}}}
    }
  ]
}
```

2. **Implement UVPrism Trait**:
```rust
impl UVPrism for MyPrism {
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        match pulse {
            UVPulse::Wavefront(wavefront) => {
                match wavefront.frequency.as_str() {
                    "process" => {
                        // Process input
                        let result = process_data(&wavefront.input)?;
                        link.emit_photon(id, result)?;
                        link.emit_trap(id, None)?;
                        Ok(true)
                    },
                    _ => Ok(false)
                }
            },
            UVPulse::Extinguish => {
                // Cleanup
                Ok(true)
            },
            _ => Ok(false)
        }
    }
}
```

3. **Add to Workspace** (`Cargo.toml`)

### Client Development

**CLI Modes**:
- **Interactive**: `uv cli` - Multi-mode interface
- **Direct**: `uv namespace:prism frequency --input data`
- **Pipe Composition**: `uv prism1 | uv prism2`

**Web Interface**: 
- **Chat Interface**: Full-featured AI chat with agent mode support
- **Component Architecture**: React components in `clients/ultraviolet-web/src/components/`
- **Service Integration**: WebSocket-based communication with UV server
- **File Context**: Drag-and-drop file management for AI context

### Core System Extensions

**Adding New Pulse Types**: Extend `UVPulse` enum in `uv-core/src/pulse.rs`

**New Communication Patterns**: Implement in `uv-core/src/link.rs`

**Transport Mechanisms**: Implement `Transport` trait for new protocols

## Project Structure Reference

```
Oppie-Toolkit/
├── uv-core/                 # Core types and functionality
├── uv-service/              # Service layer
├── clients/
│   ├── cli/                 # Command-line interface
│   ├── lambda/              # AWS Lambda client
│   └── ultraviolet-web/     # React web interface
├── prisms/
│   ├── ai/                  # AI integration prisms
│   │   ├── context/         # Prompt enrichment and AI responses
│   │   ├── agent/           # AI orchestration
│   │   ├── knowledge/       # Knowledge storage/retrieval
│   │   └── mcp/             # Model Context Protocol integration
│   ├── aws/                 # AWS service integrations
│   ├── core/                # Core system prisms
│   └── system/              # System utilities
├── docs/                    # Documentation and specifications
└── scripts/                 # Build and deployment scripts
```

## Common Development Tasks

### Task A: New Functionality Prisms
**Use Case**: Adding domain-specific functionality (e.g., data processing, external API integration)

**Steps**:
1. Create prism directory: `prisms/{namespace}/{name}/`
2. Define `spectrum.json` with wavelengths
3. Implement `src/lib.rs` with `UVPrism` trait
4. Add `Cargo.toml` and register in workspace
5. Test via CLI: `uv {namespace}:{name} {frequency}`

### Task B: Client Updates
**CLI Enhancements**: Modify `clients/cli/src/` modules
- `interactive/` - Interactive mode improvements
- `parsing/` - Command parsing logic
- `rendering/` - Output formatting

**Web UI Updates**: React components in `clients/ultraviolet-web/src/`
- **Chat Interface**: `components/ChatView.js` - Main chat interface
- **Agent Integration**: `services/AgentChatService.js` - Agent mode handling
- **File Management**: `components/FileContextPanel.js` - Context file handling
- **Conversation History**: `components/ConversationHistory.js` - Session management
- **Service Layer**: `src/services/` - WebSocket and API communication

### Task C: System Evolution
**Composition Features**: 
- Extend pulse protocol for pipeline definitions
- Add composition engine to `uv-service`
- Update CLI for pipeline syntax

**Parallel Execution**:
- Extend `PrismMultiplexer` for concurrent prism execution
- Add synchronization primitives to pulse protocol
- Update spectrum format for parallel capability declarations

## Current Limitations & Future Directions

### Implemented
- Thread-based prism execution
- Basic pulse protocol (Wavefront/Photon/Trap/Extinguish)
- Refraction system for dependencies
- CLI with multiple interaction modes
- AI integration through context enrichment
- Schema validation for inputs/outputs

### Planned/Future
- **Prism Composition**: Pipeline definitions and external composition
- **Parallel Execution**: Concurrent prism processing
- **Advanced Property Mapping**: Transformations and conditionals
- **Distributed Execution**: Remote prism execution across network
- **Enhanced AI Orchestration**: More sophisticated agent capabilities
- **Process Management**: Long-running and scheduled operations

### Extension Points
- **New Transport Mechanisms**: WebSocket, message queues
- **Advanced Scheduling**: Cron-like capabilities
- **Monitoring & Observability**: Metrics and tracing
- **Security**: Authentication and authorization
- **Registry System**: Prism discovery and versioning
- **Remote Execution**: Enhanced distributed computing capabilities
- **Web Interface**: Additional UI components and features

## Key Implementation Files

**Core Types**: `uv-core/src/lib.rs` - Main type exports
**Pulse Protocol**: `uv-core/src/pulse.rs` - Communication types
**Spectrum Format**: `uv-core/src/spectrum.rs` - Metadata definitions
**Link Communication**: `uv-core/src/link.rs` - Thread-safe channels
**CLI Entry**: `clients/cli/src/main.rs` - Command-line interface
**AI Context**: `prisms/ai/context/src/spectrum.rs` - AI prompt templates

## Client Agnosticism & Dynamic Discovery

**Critical Architectural Principle**: UV clients (CLI, Web, Lambda) are completely agnostic to specific prism implementations. They operate purely through dynamic discovery and spectrum-driven interaction.

### Dynamic CLI Behavior
The CLI dynamically adapts to any prism without hardcoded knowledge:
- **Argument Mapping**: CLI arguments are dynamically mapped to prism input schemas
- **Help Generation**: Help text is generated from spectrum descriptions and input schemas
- **Output Rendering**: Response formatting is inferred from spectrum output schemas
- **Validation**: Input validation uses spectrum-defined JSON schemas

### Example CLI Flow
```bash
# CLI discovers prism capabilities dynamically
$ uv aws:burner --help
# Help generated from aws:burner spectrum.json

$ uv aws:burner create --name test-account
# Arguments mapped to spectrum input schema
# No hardcoded knowledge of burner prism in CLI
```

### Web Interface Discovery
The web client similarly operates through dynamic discovery:
- **Prism Explorer**: Dynamically lists available prisms via `system:discovery`
- **Form Generation**: Input forms generated from spectrum schemas
- **Result Rendering**: Output display inferred from spectrum definitions
- **Agent Integration**: AI agents discover capabilities through spectrum metadata

### Anti-Pattern Warning
**DO NOT** add prism-specific logic to clients. Common mistakes to avoid:
- Hardcoding prism names or behaviors in CLI/Web code
- Creating prism-specific UI components
- Adding prism-specific argument parsing
- Implementing prism-specific rendering logic

### Correct Approach
All prism interaction should be:
1. **Schema-Driven**: Use spectrum definitions for all client behavior
2. **Discovery-Based**: Use `system:discovery` to find available prisms
3. **Generic**: Client code should work with any prism following spectrum format
4. **Self-Describing**: Prisms provide all necessary metadata through their spectrum

This architecture ensures:
- **Extensibility**: New prisms work immediately without client changes
- **Consistency**: All prisms have uniform client interaction patterns
- **Maintainability**: Client code remains simple and generic
- **Flexibility**: Prisms can evolve without breaking client compatibility

## Development Philosophy

1. **Progressive Enhancement**: Each phase builds on previous, providing immediate value
2. **Schema-First Design**: All communication is strongly typed and validated
3. **Self-Description**: Components describe their capabilities for discovery
4. **Thread Isolation**: Clean separation and stability through dedicated threads
5. **Explicit Dependencies**: Clear declaration of inter-prism relationships
6. **AI-Native**: Designed from ground up for AI orchestration and natural language interaction
7. **Client Agnosticism**: Clients operate purely through dynamic discovery, never hardcoded prism knowledge

---

This context should provide any AI assistant with comprehensive understanding of the Ultraviolet system's architecture, concepts, and development patterns. The system represents a novel approach to distributed computing that bridges human and AI interaction through composable, schema-driven functional units.
