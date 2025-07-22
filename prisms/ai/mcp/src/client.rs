use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::spectrum::{McpServerConfig, McpTool, McpResource, McpResourceContent, McpToolContent};
use uv_core::{Result, UVError};

/// MCP client for communicating with MCP servers via stdio
pub struct McpClient {
    process: Child,
    request_id: u64,
}

impl McpClient {
    /// Create a new MCP client and connect to server
    pub async fn connect(config: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Set working directory if specified
        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        }

        let process = cmd.spawn()
            .map_err(|e| UVError::ExecutionError(format!("Failed to spawn MCP server: {}", e)))?;

        let mut client = Self {
            process,
            request_id: 1,
        };

        // Initialize the connection
        client.initialize().await?;

        Ok(client)
    }

    /// Initialize the MCP connection
    async fn initialize(&mut self) -> Result<()> {
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "clientInfo": {
                    "name": "uv-mcp-client",
                    "version": "0.1.0"
                }
            }
        });

        self.send_request(init_request).await?;
        let _response = self.read_response().await?;

        // Send initialized notification
        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        self.send_request(initialized).await?;
        Ok(())
    }

    /// List available tools
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "tools/list"
        });

        self.send_request(request).await?;
        let response = self.read_response().await?;

        let tools = response["result"]["tools"]
            .as_array()
            .ok_or_else(|| UVError::ExecutionError("Invalid tools response".to_string()))?;

        let mut result = Vec::new();
        for tool in tools {
            let mcp_tool: McpTool = serde_json::from_value(tool.clone())
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse tool: {}", e)))?;
            result.push(mcp_tool);
        }

        Ok(result)
    }

    /// List available resources
    pub async fn list_resources(&mut self) -> Result<Vec<McpResource>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "resources/list"
        });

        self.send_request(request).await?;
        let response = self.read_response().await?;

        let resources = response["result"]["resources"]
            .as_array()
            .ok_or_else(|| UVError::ExecutionError("Invalid resources response".to_string()))?;

        let mut result = Vec::new();
        for resource in resources {
            let mcp_resource: McpResource = serde_json::from_value(resource.clone())
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse resource: {}", e)))?;
            result.push(mcp_resource);
        }

        Ok(result)
    }

    /// Call a tool
    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<(Vec<McpToolContent>, bool)> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        });

        self.send_request(request).await?;
        let response = self.read_response().await?;

        let is_error = response.get("error").is_some();
        
        if is_error {
            let error_msg = response["error"]["message"]
                .as_str()
                .unwrap_or("Unknown MCP error");
            return Ok((vec![McpToolContent {
                content_type: "text".to_string(),
                text: format!("Error: {}", error_msg),
            }], true));
        }

        let content = response["result"]["content"]
            .as_array()
            .ok_or_else(|| UVError::ExecutionError("Invalid tool call response".to_string()))?;

        let mut result = Vec::new();
        for item in content {
            let tool_content: McpToolContent = serde_json::from_value(item.clone())
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse tool content: {}", e)))?;
            result.push(tool_content);
        }

        Ok((result, false))
    }

    /// Read a resource
    pub async fn read_resource(&mut self, uri: &str) -> Result<Vec<McpResourceContent>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "resources/read",
            "params": {
                "uri": uri
            }
        });

        self.send_request(request).await?;
        let response = self.read_response().await?;

        let contents = response["result"]["contents"]
            .as_array()
            .ok_or_else(|| UVError::ExecutionError("Invalid resource read response".to_string()))?;

        let mut result = Vec::new();
        for content in contents {
            let resource_content: McpResourceContent = serde_json::from_value(content.clone())
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse resource content: {}", e)))?;
            result.push(resource_content);
        }

        Ok(result)
    }

    /// Send a JSON-RPC request
    async fn send_request(&mut self, request: Value) -> Result<()> {
        let stdin = self.process.stdin.as_mut()
            .ok_or_else(|| UVError::ExecutionError("No stdin available".to_string()))?;

        let request_str = serde_json::to_string(&request)
            .map_err(|e| UVError::ExecutionError(format!("Failed to serialize request: {}", e)))?;

        stdin.write_all(format!("{}\n", request_str).as_bytes()).await
            .map_err(|e| UVError::ExecutionError(format!("Failed to write to stdin: {}", e)))?;

        stdin.flush().await
            .map_err(|e| UVError::ExecutionError(format!("Failed to flush stdin: {}", e)))?;

        Ok(())
    }

    /// Read a JSON-RPC response
    async fn read_response(&mut self) -> Result<Value> {
        let stdout = self.process.stdout.as_mut()
            .ok_or_else(|| UVError::ExecutionError("No stdout available".to_string()))?;

        let mut reader = BufReader::new(stdout);
        
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await
                .map_err(|e| UVError::ExecutionError(format!("Failed to read from stdout: {}", e)))?;

            if line.trim().is_empty() {
                return Err(UVError::ExecutionError("Empty response from MCP server".to_string()));
            }

            let response: Value = serde_json::from_str(&line)
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse JSON response: {}", e)))?;

            // Skip notifications, only return responses with IDs
            if response.get("id").is_some() || response.get("error").is_some() {
                return Ok(response);
            }
            // Otherwise, continue reading (skip notifications)
        }
    }

    /// Get next request ID
    fn next_id(&mut self) -> u64 {
        let id = self.request_id;
        self.request_id += 1;
        id
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}