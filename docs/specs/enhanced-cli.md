# Enhanced CLI Interface

## Overview

The Ultraviolet CLI provides a powerful command-line interface for interacting with the UV Service API. It serves as one of several client interfaces to the distributed Ultraviolet platform as described in the [Distributed Architecture](distributed-architecture.md) specification. Beyond basic command execution, it offers rich rendering capabilities that automatically format output based on data structure. This document outlines how the CLI infers UI components and renders them in a terminal environment.

## Architecture

```
                     +---------------+
                     |               |
User Commands -----> | Command Parser | 
                     |               |
                     +---------------+
                            |
                            v
                     +---------------+
                     |               |
                     | UV Service    |
                     | Client        |
                     +---------------+
                            |
                            v
                     +---------------+
                     |               |
                     | WebSocket     |
                     | Connection    |
                     +---------------+
                            |
                            v
                     +---------------+
                     |               |
                     | Data Analyzer |
                     |               |
                     +---------------+
                            |
                            v
                     +---------------+
                     |               |
                     | UI Renderer   |
                     |               |
                     +---------------+
                            |
                            v
                     Terminal Output
```

The CLI connects to a local or remote UV Service via WebSocket, using the same [Pulse Protocol](pulse-protocol.md) that powers the entire system.

## Core Components

### 1. Command Parser

- Parses command-line arguments
- Validates input against spectrum schemas
- Handles flags and options
- Provides rich help and documentation

### 2. UV Service Client

- Connects to the local or remote UV Service
- Manages WebSocket connection lifecycle
- Handles reconnection and error recovery
- Service discovery and selection

### 3. Data Analyzer

- Examines response structure
- Identifies data types and patterns
- Determines appropriate rendering approach
- Makes inferences about rendering intent

### 4. UI Renderer

- Terminal-optimized rendering engine
- Rich text formatting capabilities
- Interactive components where appropriate
- Adaptive to terminal capabilities

## Rendering Capabilities

### Tables

For arrays of objects:

```
┌──────────┬────────────┬──────────┐
│ Name     │ ID         │ Status   │
├──────────┼────────────┼──────────┤
│ example1 │ 123        │ active   │
│ example2 │ 456        │ inactive │
└──────────┴────────────┴──────────┘
```

Features:
- Column auto-sizing
- Header highlighting
- Value formatting
- Paging for large datasets
- Optional sorting

### Cards/Records

For single objects:

```
╔══════════════════════════════════╗
║ Account Details                  ║
╠═══════════════╤══════════════════╣
║ Name          │ example1         ║
╟───────────────┼──────────────────╢
║ ID            │ 123              ║
╟───────────────┼──────────────────╢
║ Status        │ active           ║
╟───────────────┼──────────────────╢
║ Created       │ Jan 1, 2025      ║
╚═══════════════╧══════════════════╝
```

Features:
- Formatted key-value pairs
- Type-sensitive value display
- Smart title generation
- Color coding

### Lists

For arrays of values:

```
• First item
• Second item
• Third item
```

Features:
- Bullet or numbered formatting
- Indentation for hierarchy
- Word wrapping
- Color coding by content type

### Progress Displays

For ongoing operations:

```
Uploading: [██████████████████████] 80% (4.2MB/5.0MB)
```

Features:
- Progress bars
- Spinners for indeterminate progress
- Speed and ETA calculations
- Multi-task tracking

### Interactive Components

For inputs and selections:

```
Select an AWS region:
> us-west-2
  us-east-1
  eu-central-1
  ap-southeast-1
```

Features:
- Keyboard navigation
- Selection highlighting
- Search filtering
- Multi-select capability

## Type-Based Enhancements

The CLI automatically enhances display based on recognized types:

| Type Pattern | CLI Enhancement |
|--------------|----------------|
| URL/URI | Underlined, clickable (in terminals that support it) |
| Date/Time | Formatted to locale, with relative time |
| File Path | Colored by existence/permission, clickable |
| Status | Color-coded (green for success/active, yellow for pending, etc.) |
| Progress | Shown as percentage or bar |
| Size | Formatted with appropriate units (KB, MB, GB) |

## Implementation Details

### Color Support

- Auto-detection of terminal color capabilities
- Graceful fallback for limited terminals
- Configurable color schemes
- Respect for NO_COLOR environment variable

### Terminal Size

- Adapts to available width and height
- Wraps content appropriately
- Handles resizing mid-display
- Paginates large outputs

### Streaming Output

- Progressive rendering of large datasets
- Live updates for streaming data
- Cursor control for in-place updates
- Rate limiting to prevent flicker

### Output Redirection

- Detection of piped output
- Plain text rendering when output is redirected
- Machine-readable formats for scripting (--json flag)
- Raw output mode for programmatic consumption

## Configuration

Users can customize CLI behavior:

```yaml
# ~/.config/uv/cli_config.yaml
rendering:
  color_mode: auto  # auto, always, never
  table_style: unicode  # unicode, ascii, markdown
  date_format: relative  # relative, iso, locale
  
terminal:
  paging: auto  # auto, always, never
  max_width: auto  # auto or number of columns
  
interaction:
  confirm_dangerous: true  # prompt for confirmation
  progress_animation: true
```

## Examples

### Basic Command Execution

```bash
$ uv aws:s3 list-buckets

Name                    Created             Region     
----------------------  ----------------   -----------
data-processing         2025-01-15         us-west-2
logs-archive            2025-01-10         us-east-1
user-uploads            2025-01-20         us-west-2
```

### Rich Output Formatting

```bash
$ uv aws:lambda list-functions

╔════════════════════════════════════════════════════════╗
║ Lambda Functions                                       ║
╚════════════════════════════════════════════════════════╝

api-handler
  Runtime: nodejs18.x
  Memory: 256 MB
  Timeout: 30s
  Last Modified: 3 days ago

data-processor
  Runtime: python3.9
  Memory: 512 MB
  Timeout: 5m
  Last Modified: yesterday

image-resizer
  Runtime: nodejs18.x
  Memory: 1024 MB
  Timeout: 2m
  Last Modified: 5 hours ago
```

### Streaming Output

```bash
$ uv ops:logs watch --service=api

[12:05:32] INFO  Server started on port 3000
[12:05:45] INFO  Connection from 192.168.1.5
[12:05:46] DEBUG Processing request /api/users
[12:05:47] WARN  Slow query detected (2.5s): SELECT * FROM users
[12:05:50] ERROR Database connection lost
[12:05:52] INFO  Database connection reestablished
...
```

### Interactive Selection

```bash
$ uv aws:ec2 start-instance

Select instances to start:
[x] i-1234567890abcdef0 (api-server-1)
[ ] i-0abcdef1234567890 (api-server-2)
[x] i-abcdef0123456789a (worker-1)
[ ] i-bcdef0123456789ab (worker-2)

Press SPACE to select/deselect, ENTER to confirm
