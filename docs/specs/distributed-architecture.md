# Distributed Architecture

## Overview

Ultraviolet is designed as a distributed computing platform that enables transparent, location-independent execution of prisms. This architecture allows prisms to execute on the most appropriate nodes in a network, based on resource requirements, data locality, and specialized hardware needs, all while maintaining a simple programming model for prism developers and users.

## Core Concepts

### Distributed Service Network

```
                    +-------------+
                    | Client Node |
                    +------+------+
                           |
                           v
            +------------------------------+
            |                              |
      +-----+-----+                  +-----+-----+
      | UV Service |<--------------->| UV Service |
      +-----------+                  +-----------+
           ^   ^                          ^  ^
           |   |                          |  |
      +----+   +----+                +----+  +----+
      |             |                |            |
+-----+----+   +----+-----+     +---+------+  +--+-------+
| GPU Prism |   | CPU Prism |    | AWS Prism |  | ML Prism |
+----------+   +----------+     +----------+  +----------+

```

1. **UV Service Instances**: The core of the distributed architecture is the UV Service, which runs as a daemon/server process on each participating node.

2. **Peer-to-Peer Connectivity**: UV Service instances connect to each other in a peer-to-peer network, forming a distributed compute fabric.

3. **Location Transparency**: Prism execution is location-transparent; the system determines the optimal execution location based on prism requirements and resource availability.

4. **Automatic Routing**: Requests are automatically routed to the most appropriate node for execution.

5. **Resource Federation**: The combined resources of all connected nodes create a unified computing environment.

## Architecture Components

### 1. UV Service

The UV Service is the core component that enables the distributed architecture:

- **Service API**: HTTP and WebSocket endpoints for clients and other services
- **Service Manager**: Handles inter-service communication
- **Prism Registry**: Maintains information about available prisms (local and remote)
- **Routing Engine**: Determines optimal execution location for prisms
- **Protocol Adapter**: Transforms API calls to native prism calls

### 2. Service Discovery

Services discover each other through:

- **Static Configuration**: Manual specification of known services
- **DNS-Based Discovery**: Using DNS SRV records for service location
- **Broadcast Discovery**: Local network discovery protocol
- **Registry Service**: Optional central registry for large deployments

### 3. Routing Layer

The routing layer determines where to execute a prism based on:

- **Prism Availability**: Where implementations are available
- **Resource Requirements**: CPU, memory, disk, etc.
- **Specialized Hardware**: GPUs, TPUs, or other accelerators
- **Data Locality**: Proximity to required data sources
- **Load Balancing**: Distribution of workload across nodes
- **Latency Requirements**: Execution time constraints

### 4. Execution Flow

When a prism refracts to another prism, the following steps occur:

1. The source prism calls `refract()` with the target prism ID
2. The local UV Service receives the refraction request
3. The routing layer determines the optimal execution location
4. If local execution is optimal, the request is processed locally
5. If remote execution is better, the request is forwarded to the appropriate UV Service
6. The target UV Service executes the prism and returns results
7. Results flow back through the service network to the caller
8. The source prism receives the results, unaware of where execution occurred

## Security Model

The distributed architecture includes a comprehensive security model:

1. **Service Authentication**: Services authenticate to each other using mTLS or API keys
2. **Authorization**: Fine-grained permission control for prism execution
3. **Encrypted Communication**: All inter-service communication is encrypted
4. **Execution Boundaries**: Configurable limits on what prisms can be executed remotely
5. **Audit Trail**: Logging of all cross-service executions

## Implementation Phases

The distributed architecture can be implemented in phases:

### Phase 1: Service API Layer

- Implement local UV Service with API endpoints
- Update CLI and other interfaces to use the service API
- Maintain local-only execution

### Phase 2: Basic Service-to-Service Communication

- Implement service discovery mechanisms
- Enable basic forwarding of prism execution between services
- Support manual configuration of service connections

### Phase 3: Intelligent Routing

- Implement the routing layer
- Add resource awareness for execution decisions
- Support specialized hardware requirements

### Phase 4: Advanced Features

- Dynamic load balancing
- Fault tolerance and failover
- Distributed caching of prism results
- Multi-tenant security model

## Client Integration

Clients (CLI, web UI, SDKs) connect to a local or remote UV Service:

1. **CLI Mode**: The CLI can optionally start a local UV Service or connect to an existing one
2. **Web Mode**: The web UI connects to a UV Service via HTTP/WebSocket
3. **SDK Mode**: Language-specific SDKs provide native API clients for the UV Service
4. **Third-Party Mode**: Any HTTP client can interact with the UV Service API

## Benefits of Distributed Architecture

This architecture provides numerous advantages:

1. **Scalability**: Easily scale by adding more nodes to the network
2. **Hardware Utilization**: Use specialized hardware where available
3. **Resource Optimization**: Execute prisms on the most appropriate nodes
4. **Resilience**: Continue operating if some nodes fail
5. **Simplicity**: Prism developers don't need to handle distribution concerns
6. **Flexibility**: Support various deployment models from single-node to large clusters

## Example Scenarios

### Scenario 1: AI Model Execution

1. A user requests an image generation via the CLI
2. The local UV Service receives the request but has no GPU
3. The request is routed to a GPU-equipped node in the network
4. The image is generated on the GPU node and returned to the user
5. The user experience is seamless, as if execution happened locally

### Scenario 2: Cross-Environment Operations

1. A prism needs to perform operations in both AWS and Azure
2. The request is split, with AWS operations routed to an AWS node
3. Azure operations are routed to an Azure node
4. Results are aggregated and returned to the caller
5. The operation appears as a single, atomic action to the user

### Scenario 3: Edge Computing

1. Data collection happens on edge devices running UV Service
2. Initial processing occurs on the edge
3. Complex analysis is routed to more powerful central nodes
4. Results are returned to the edge for action
5. The entire process appears as a single operation

## Future Directions

The distributed architecture enables several future capabilities:

1. **Global Prism Registry**: Discover and use prisms from a global network
2. **Prism Marketplace**: Ecosystem for sharing and monetizing prisms
3. **Edge-to-Cloud Continuum**: Seamless execution from edge devices to cloud
4. **Hybrid Environments**: Bridge between on-prem and cloud resources

## Configuration Example

```yaml
# Example configuration for a UV Service node
service:
  id: "node-gpu-01"
  listen: "0.0.0.0:8080"
  
discovery:
  method: "static"  # static, dns, broadcast, registry
  static_nodes:
    - "https://uv-service.internal.example.com:8080"
    - "https://uv-gpu-node.internal.example.com:8080"
  
routing:
  local_preference: 0.8  # Preference for local execution (0-1)
  capabilities:
    - "gpu:nvidia:a100"
    - "memory:128gb"
  
security:
  auth_required: true
  cert_file: "/etc/uv/server.crt"
  key_file: "/etc/uv/server.key"
  ca_file: "/etc/uv/ca.crt"
