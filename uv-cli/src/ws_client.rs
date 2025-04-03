//! WebSocket client implementation for the UV CLI.
//!
//! This module provides a WebSocket client for connecting to UV services,
//! both remote and embedded.

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Result, anyhow};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, debug};
use url::Url;
use uuid::Uuid;

use uv_core::{UVPulse, Wavefront, Photon, Trap};
use uv_service::{ServiceOptions, start_service};

/// WebSocket client for connecting to UV services
pub struct WebSocketClient {
    /// The URL of the WebSocket server
    url: String,
    
    /// Whether to use secure WebSocket (wss://)
    secure: bool,
    
    /// Timeout for WebSocket operations
    timeout: Duration,
}

impl WebSocketClient {
    /// Create a new WebSocket client
    pub fn new(host: &str, secure: bool) -> Self {
        Self {
            url: host.to_string(),
            secure,
            timeout: Duration::from_secs(30),
        }
    }
    
    /// Execute a command via WebSocket
    pub async fn execute(
        &self, 
        prism: &str, 
        frequency: &str, 
        args: Value
    ) -> Result<Value> {
        // Format the WebSocket URL
        let scheme = if self.secure { "wss" } else { "ws" };
        let ws_url = format!("{}://{}/ws", scheme, self.url);
        
        debug!("Connecting to WebSocket at {}", ws_url);
        
        // Connect to the WebSocket server
        let url = Url::parse(&ws_url).map_err(|e| anyhow!("Invalid URL: {}", e))?;
        let (ws_stream, _) = tokio::time::timeout(
            self.timeout,
            connect_async(url)
        ).await?.map_err(|e| anyhow!("WebSocket connection error: {}", e))?;
        
        debug!("WebSocket connection established");
        
        // Split the WebSocket stream
        let (mut write, mut read) = ws_stream.split();
        
        // Create a wavefront
        let id = Uuid::new_v4();
        let wavefront = UVPulse::Wavefront(Wavefront {
            id,
            prism: prism.to_string(),
            frequency: frequency.to_string(),
            input: args,
        });
        
        // Serialize the wavefront
        let wavefront_json = serde_json::to_string(&wavefront)
            .map_err(|e| anyhow!("Failed to serialize wavefront: {}", e))?;
        
        debug!("Sending wavefront: {}", wavefront_json);
        
        // Send the wavefront
        write.send(Message::Text(wavefront_json)).await
            .map_err(|e| anyhow!("Failed to send wavefront: {}", e))?;
        
        // Collect responses
        let mut results = Vec::new();
        
        // Process responses until we get a trap
        loop {
            match tokio::time::timeout(self.timeout, read.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    debug!("Received message: {}", text);
                    
                    // Parse the message
                    let pulse: UVPulse = serde_json::from_str(&text)
                        .map_err(|e| anyhow!("Failed to parse message: {}", e))?;
                    
                    match pulse {
                        UVPulse::Photon(Photon { id: _, data }) => {
                            // Add the photon data to the results
                            results.push(data);
                        },
                        UVPulse::Trap(Trap { id: _, error }) => {
                            // If there's an error in the trap, return it
                            if let Some(err) = error {
                                return Err(anyhow!("Prism error: {}", err));
                            }
                            
                            // Otherwise we're done
                            debug!("Received trap, command complete");
                            
                            // Send a proper closing frame
                            debug!("Sending WebSocket close frame");
                            if let Err(e) = write.send(Message::Close(None)).await {
                                debug!("Error sending close frame: {}", e);
                                // Continue anyway, as we're closing
                            }
                            
                            // Wait a short time for the close to be processed
                            tokio::time::sleep(Duration::from_millis(50)).await;
                            
                            break;
                        },
                        _ => {
                            debug!("Unexpected pulse type: {:?}", pulse);
                        }
                    }
                },
                Ok(Some(Ok(_))) => {
                    // Ignore non-text messages
                },
                Ok(Some(Err(e))) => {
                    return Err(anyhow!("WebSocket error: {}", e));
                },
                Ok(None) => {
                    return Err(anyhow!("WebSocket closed unexpectedly"));
                },
                Err(_) => {
                    return Err(anyhow!("WebSocket timeout"));
                }
            }
        }
        
        // Convert results to JSON
        if results.len() == 1 {
            // If only one result, return it directly
            Ok(results.pop().unwrap())
        } else {
            // Otherwise return an array
            Ok(Value::Array(results))
        }
    }
}

/// Embedded service manager
pub struct EmbeddedService {
    /// The handle to the service task
    handle: JoinHandle<()>,
    
    /// The address the service is bound to
    address: SocketAddr,
}

impl EmbeddedService {
    /// Start an embedded service and return a handle to it
    pub async fn start() -> Result<Self> {
        // Find an available port
        let port = portpicker::pick_unused_port().ok_or_else(|| anyhow!("No available ports"))?;
        let address = format!("127.0.0.1:{}", port).parse::<SocketAddr>()
            .map_err(|e| anyhow!("Invalid address: {}", e))?;
        
        debug!("Starting embedded service on port {}", port);
        
        // Determine log level based on parent process settings
        let log_level = if tracing::enabled!(tracing::Level::DEBUG) {
            uv_service::LogLevel::Debug
        } else if tracing::enabled!(tracing::Level::ERROR) && !tracing::enabled!(tracing::Level::WARN) {
            uv_service::LogLevel::Quiet
        } else {
            uv_service::LogLevel::Normal
        };

        // Create service options
        let options = ServiceOptions {
            bind_address: address,
            enable_tls: false,
            cert_path: None,
            key_path: None,
            serve_static: false,
            static_dir: None,
            init_tracing: false, // Don't initialize tracing in embedded mode
            log_level,
        };
        
        // Clone the address for the task
        let task_address = address;
        
        // Start the service in a separate task
        let handle = tokio::spawn(async move {
            if let Err(e) = start_service(options).await {
                error!("Embedded service error: {}", e);
            }
        });
        
        // Wait for the service to be ready
        wait_for_service_ready(task_address).await?;
        
        Ok(Self {
            handle,
            address,
        })
    }
    
    /// Get the URL of the embedded service
    pub fn url(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }
}

impl Drop for EmbeddedService {
    fn drop(&mut self) {
        // Abort the service task when dropped
        debug!("Stopping embedded service");
        
        // Give a small amount of time for any pending operations to complete
        // This helps ensure the WebSocket close handshake can complete
        let _ = self.handle.abort();
    }
}

/// Wait for a service to be ready
async fn wait_for_service_ready(address: SocketAddr) -> Result<()> {
    // Try connecting for up to 5 seconds
    let start = std::time::Instant::now();
    let max_wait = Duration::from_secs(5);
    
    while start.elapsed() < max_wait {
        match tokio::net::TcpStream::connect(address).await {
            Ok(_) => {
                // Successfully connected, service is ready
                debug!("Service is ready");
                return Ok(());
            },
            Err(_) => {
                // Service not ready yet, wait a bit
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    
    Err(anyhow!("Service failed to start within timeout"))
}

/// Execute a command using an embedded service
pub async fn execute_with_embedded(
    prism: &str, 
    frequency: &str, 
    args: &[String]
) -> Result<Value> {
    // Start the embedded service
    let service = EmbeddedService::start().await?;
    
    // Create a WebSocket client
    let client = WebSocketClient::new(&service.url(), false);
    
    // Process arguments
    let args_json = process_args(args);
    
    // Execute the command
    client.execute(prism, frequency, args_json).await
}

/// Execute a command against a remote service
pub async fn execute_remote(
    remote: &str,
    secure: bool,
    prism: &str,
    frequency: &str,
    args: &[String]
) -> Result<Value> {
    // Create a WebSocket client
    let client = WebSocketClient::new(remote, secure);
    
    // Process arguments
    let args_json = process_args(args);
    
    // Execute the command
    client.execute(prism, frequency, args_json).await
}

/// Process arguments into a JSON object
fn process_args(args: &[String]) -> Value {
    let mut result = serde_json::json!({});
    
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        
        if arg.starts_with("--") {
            let key = &arg[2..];
            
            // Check for --key=value format
            if let Some(eq_pos) = key.find('=') {
                let (real_key, value) = key.split_at(eq_pos);
                result[real_key] = serde_json::json!(value[1..]);
            } 
            // Otherwise expect --key value format
            else if i + 1 < args.len() {
                result[key] = serde_json::json!(args[i + 1]);
                i += 1; // Skip next arg since we used it as value
            } else {
                // Treat as boolean flag if no value
                result[key] = serde_json::json!(true);
            }
        } else {
            // Add positional arguments with numeric keys
            // Skip the first one which is the command name
            if i > 0 {
                result[format!("arg{}", i)] = serde_json::json!(arg);
            }
        }
        
        i += 1;
    }
    
    result
}
