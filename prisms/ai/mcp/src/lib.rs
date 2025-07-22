//! MCP (Model Context Protocol) prism implementation for the Ultraviolet system.
//!
//! This crate provides an MCP client prism that allows interaction with MCP servers
//! for accessing external tools and resources.

pub mod spectrum;
pub mod client;

use std::collections::HashMap;
use serde_json::{json, Value};
use tokio::runtime::Runtime;
use uuid::Uuid;

use spectrum::{
    McpConfig, ListToolsRequest, ListResourcesRequest, CallToolRequest, ReadResourceRequest,
    McpTool, McpResource
};
use client::McpClient;
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// MCP prism for accessing Model Context Protocol servers
pub struct McpPrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
    /// Tokio runtime for async operations
    runtime: Runtime,
    /// MCP server configurations
    config: Option<McpConfig>,
}

impl McpPrism {
    /// Create a new MCP prism
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new()
            .map_err(|e| UVError::ExecutionError(format!("Failed to create runtime: {}", e)))?;

        Ok(Self {
            spectrum: None,
            runtime,
            config: None,
        })
    }

    /// Load MCP configuration from ~/.uv/mcp.json
    fn load_config(&mut self) -> Result<()> {
        if self.config.is_some() {
            return Ok(());
        }

        let home_dir = dirs::home_dir()
            .ok_or_else(|| UVError::ExecutionError("Could not determine home directory".to_string()))?;

        let config_path = home_dir.join(".uv").join("mcp.json");
        
        if !config_path.exists() {
            return Err(UVError::ExecutionError(format!(
                "MCP configuration file not found at {}. Please create it with your MCP server configurations.",
                config_path.display()
            )));
        }

        let config_content = std::fs::read_to_string(&config_path)
            .map_err(|e| UVError::ExecutionError(format!("Failed to read MCP config: {}", e)))?;

        let config: McpConfig = serde_json::from_str(&config_content)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse MCP config: {}", e)))?;

        self.config = Some(config);
        Ok(())
    }

    /// Get server configuration by name
    fn get_server_config(&self, server_name: &str) -> Result<&spectrum::McpServerConfig> {
        let config = self.config.as_ref()
            .ok_or_else(|| UVError::ExecutionError("MCP configuration not loaded".to_string()))?;

        config.mcp_servers.get(server_name)
            .ok_or_else(|| UVError::ExecutionError(format!("MCP server '{}' not found in configuration", server_name)))
    }

    /// Handle 'list_tools' frequency
    fn handle_list_tools(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: ListToolsRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        let server_config = self.get_server_config(&request.server)?;

        let tools = self.runtime.block_on(async {
            let mut client = McpClient::connect(server_config).await?;
            client.list_tools().await
        })?;

        link.emit_photon(id, json!({ "tools": tools }))?;
        link.emit_trap(id, None)?;
        Ok(())
    }

    /// Handle 'list_resources' frequency
    fn handle_list_resources(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: ListResourcesRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        let server_config = self.get_server_config(&request.server)?;

        let resources = self.runtime.block_on(async {
            let mut client = McpClient::connect(server_config).await?;
            client.list_resources().await
        })?;

        link.emit_photon(id, json!({ "resources": resources }))?;
        link.emit_trap(id, None)?;
        Ok(())
    }

    /// Handle 'call_tool' frequency
    fn handle_call_tool(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: CallToolRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        let server_config = self.get_server_config(&request.server)?;

        let (content, is_error) = self.runtime.block_on(async {
            let mut client = McpClient::connect(server_config).await?;
            client.call_tool(&request.name, request.arguments).await
        })?;

        link.emit_photon(id, json!({ 
            "content": content,
            "isError": is_error
        }))?;
        link.emit_trap(id, None)?;
        Ok(())
    }

    /// Handle 'read_resource' frequency
    fn handle_read_resource(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: ReadResourceRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;

        let server_config = self.get_server_config(&request.server)?;

        let contents = self.runtime.block_on(async {
            let mut client = McpClient::connect(server_config).await?;
            client.read_resource(&request.uri).await
        })?;

        link.emit_photon(id, json!({ "contents": contents }))?;
        link.emit_trap(id, None)?;
        Ok(())
    }

    /// Handle 'list_all_tools' frequency
    fn handle_list_all_tools(&self, id: Uuid, _input: Value, link: &UVLink) -> Result<()> {
        let config = self.config.as_ref()
            .ok_or_else(|| UVError::ExecutionError("MCP configuration not loaded".to_string()))?;

        let mut all_tools: HashMap<String, Vec<McpTool>> = HashMap::new();

        for (server_name, server_config) in &config.mcp_servers {
            match self.runtime.block_on(async {
                let mut client = McpClient::connect(server_config).await?;
                client.list_tools().await
            }) {
                Ok(tools) => {
                    all_tools.insert(server_name.clone(), tools);
                },
                Err(_) => {
                    // Skip servers that fail to connect
                    continue;
                }
            }
        }

        link.emit_photon(id, json!({ "servers": all_tools }))?;
        link.emit_trap(id, None)?;
        Ok(())
    }

    /// Handle 'list_all_resources' frequency
    fn handle_list_all_resources(&self, id: Uuid, _input: Value, link: &UVLink) -> Result<()> {
        let config = self.config.as_ref()
            .ok_or_else(|| UVError::ExecutionError("MCP configuration not loaded".to_string()))?;

        let mut all_resources: HashMap<String, Vec<McpResource>> = HashMap::new();

        for (server_name, server_config) in &config.mcp_servers {
            match self.runtime.block_on(async {
                let mut client = McpClient::connect(server_config).await?;
                client.list_resources().await
            }) {
                Ok(resources) => {
                    all_resources.insert(server_name.clone(), resources);
                },
                Err(_) => {
                    // Skip servers that fail to connect
                    continue;
                }
            }
        }

        link.emit_photon(id, json!({ "servers": all_resources }))?;
        link.emit_trap(id, None)?;
        Ok(())
    }
}

impl UVPrism for McpPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        self.load_config()?;
        Ok(())
    }

    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "list_tools" => {
                    self.handle_list_tools(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "list_resources" => {
                    self.handle_list_resources(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "call_tool" => {
                    self.handle_call_tool(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "read_resource" => {
                    self.handle_read_resource(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "list_all_tools" => {
                    self.handle_list_all_tools(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "list_all_resources" => {
                    self.handle_list_all_resources(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                _ => {
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(McpPrism::new().unwrap())
}