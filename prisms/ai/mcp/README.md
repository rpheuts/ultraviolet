# MCP Prism (ai:mcp)

The MCP prism provides integration with Model Context Protocol (MCP) servers, allowing the UV system to access external tools and resources through the standardized MCP protocol.

## Configuration

Create a configuration file at `~/.uv/mcp.json` with your MCP server definitions:

```json
{
  "mcpServers": {
    "amazon-internal-mcp-server": {
      "command": "amzn-mcp",
      "args": [],
      "env": {},
      "working_dir": "/opt/amazon-tools"
    },
    "local-filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
      "env": {}
    }
  }
}
```

## Available Frequencies

### Server-Specific Operations

- **`list_tools`** - List available tools from a specific server
  ```json
  {"server": "amazon-internal-mcp-server"}
  ```

- **`list_resources`** - List available resources from a specific server
  ```json
  {"server": "amazon-internal-mcp-server"}
  ```

- **`call_tool`** - Execute a tool on a specific server
  ```json
  {
    "server": "amazon-internal-mcp-server",
    "name": "query_tickets",
    "arguments": {"status": "open"}
  }
  ```

- **`read_resource`** - Read a resource from a specific server
  ```json
  {
    "server": "local-filesystem", 
    "uri": "file:///tmp/data.txt"
  }
  ```

### Aggregate Operations

- **`list_all_tools`** - List tools from all configured servers
  ```json
  {}
  ```

- **`list_all_resources`** - List resources from all configured servers
  ```json
  {}
  ```

## Integration with AI Agent

The MCP prism integrates seamlessly with the AI agent workflow. The context prism can call `list_all_tools` to include MCP capabilities in the AI's context, and the agent can execute MCP operations like any other prism action:

```json
{
  "prism": "ai:mcp",
  "frequency": "call_tool",
  "input": {
    "server": "amazon-internal-mcp-server",
    "name": "create_ticket",
    "arguments": {
      "title": "System Issue",
      "description": "Database connection timeout"
    }
  }
}
```

## Transport Support

Currently supports **stdio transport** only (spawning local processes). The MCP client:

1. Spawns the configured command as a subprocess
2. Communicates via JSON-RPC over stdin/stdout
3. Handles the MCP initialization handshake
4. Automatically cleans up processes when done

## Error Handling

- Missing configuration file provides helpful error message
- Unknown servers return configuration errors
- Failed server connections are gracefully skipped in aggregate operations
- MCP protocol errors are returned as structured responses