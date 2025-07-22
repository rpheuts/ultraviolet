use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub working_dir: Option<String>,
}

/// Root configuration structure
#[derive(Debug, Deserialize)]
pub struct McpConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// Request to list tools from a server
#[derive(Debug, Deserialize)]
pub struct ListToolsRequest {
    pub server: String,
}

/// Request to list resources from a server
#[derive(Debug, Deserialize)]
pub struct ListResourcesRequest {
    pub server: String,
}

/// Request to call a tool
#[derive(Debug, Deserialize)]
pub struct CallToolRequest {
    pub server: String,
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// Request to read a resource
#[derive(Debug, Deserialize)]
pub struct ReadResourceRequest {
    pub server: String,
    pub uri: String,
}

/// MCP Tool definition
#[derive(Debug, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// MCP Resource definition
#[derive(Debug, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

/// MCP Resource content
#[derive(Debug, Serialize, Deserialize)]
pub struct McpResourceContent {
    pub uri: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub text: String,
}

/// MCP Tool call result content
#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}