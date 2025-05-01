//! Remote bridge prism implementation for the Ultraviolet system.
//!
//! This prism provides a bridge to remote Ultraviolet instances, allowing
//! local prisms to send requests to remote prisms over WebSockets.

pub mod spectrum;

use futures::{SinkExt, StreamExt};
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, WebSocketStream, tungstenite::protocol::Message};
use uuid::Uuid;
use url::Url;
use serde_json::{json, Value};

use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum, Wavefront, Photon, Trap
};

use crate::spectrum::RefractRequest;

// Type alias for the WebSocket connection
type WebSocketTx = futures::stream::SplitSink<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message
>;

type WebSocketRx = futures::stream::SplitStream<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
>;

/// RemoteBridgePrism for connecting to remote Ultraviolet instances.
pub struct RemoteBridgePrism {
    spectrum: Option<UVSpectrum>,
    runtime: Runtime,
}

impl RemoteBridgePrism {
    /// Create a new remote bridge prism.
    pub fn new() -> Result<Self> {
        // Create a tokio runtime for async operations
        let runtime = Runtime::new()
            .map_err(|e| UVError::ExecutionError(format!("Failed to create runtime: {}", e)))?;
        
        Ok(Self {
            spectrum: None,
            runtime,
        })
    }
    
    /// Connect to a remote Ultraviolet WebSocket server
    async fn connect_to_server(url_str: &str) -> Result<(WebSocketTx, WebSocketRx)> {
        // Parse the URL
        let url = Url::parse(url_str)
            .map_err(|e| UVError::ExecutionError(format!("Invalid URL: {}", e)))?;
        
        // Connect to the WebSocket server
        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| UVError::ExecutionError(format!("Failed to connect: {}", e)))?;
        
        // Split the WebSocket stream into sender and receiver
        let (sender, receiver) = ws_stream.split();
        
        Ok((sender, receiver))
    }
    
    /// Handle refract frequency
    fn handle_refract(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: RefractRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid request format: {}", e)))?;
            
        // Process the request using the runtime
        self.runtime.block_on(async {
            // Connect to the remote server
            let (mut sender, mut receiver) = match Self::connect_to_server(&request.url).await {
                Ok(conn) => conn,
                Err(e) => {
                    // If connection fails, emit a trap with the error
                    link.emit_trap(id, Some(e))?;
                    return Ok(());
                }
            };
            
            // Create a remote wavefront
            let remote_id = Uuid::new_v4();
            let wavefront = UVPulse::Wavefront(Wavefront {
                id: remote_id,
                prism: request.prism,
                frequency: request.frequency,
                input: request.input.or(Some(json!({}))).unwrap(),
            });
            
            // Serialize and send the wavefront
            let wavefront_json = serde_json::to_string(&wavefront)
                .map_err(|e| UVError::ExecutionError(format!("Failed to serialize wavefront: {}", e)))?;
            
            sender.send(Message::Text(wavefront_json)).await
                .map_err(|e| UVError::ExecutionError(format!("Failed to send wavefront: {}", e)))?;
            
            // Process responses until we get a trap
            loop {
                match receiver.next().await {
                    Some(Ok(Message::Text(text))) => {
                        // Parse the response as a UVPulse
                        let pulse: UVPulse = serde_json::from_str(&text)
                            .map_err(|e| UVError::ExecutionError(format!("Invalid response format: {}", e)))?;
                        
                        match pulse {
                            UVPulse::Photon(Photon { data, .. }) => {
                                // Forward the photon to the local client
                                link.emit_photon(id, data)?;
                            },
                            UVPulse::Trap(Trap { error, .. }) => {
                                // If there's an error in the trap, print it to stderr
                                if let Some(err) = &error {
                                    eprintln!("Error from remote: {}", err);
                                }
                                
                                // Forward the trap to the local client
                                link.emit_trap(id, error)?;
                                
                                // Send a close frame to properly terminate the WebSocket connection
                                sender.send(Message::Close(None)).await
                                    .map_err(|e| UVError::ExecutionError(format!("Failed to send close frame: {}", e)))?;
                                
                                break;
                            },
                            _ => {
                                // Ignore other pulse types
                                continue;
                            }
                        }
                    },
                    Some(Ok(Message::Close(_))) => {
                        // Connection closed by server without a trap
                        link.emit_trap(id, Some(UVError::ExecutionError(
                            "Connection closed by server without completion signal".into()
                        )))?;
                        break;
                    },
                    Some(Ok(_)) => {
                        // Ignore other WebSocket message types
                        continue;
                    },
                    Some(Err(e)) => {
                        // WebSocket error
                        link.emit_trap(id, Some(UVError::ExecutionError(
                            format!("WebSocket error: {}", e)
                        )))?;
                        break;
                    },
                    None => {
                        // Stream ended without a trap
                        link.emit_trap(id, Some(UVError::ExecutionError(
                            "Connection closed unexpectedly".into()
                        )))?;
                        break;
                    }
                }
            }
            
            Ok(())
        })
    }
}

impl UVPrism for RemoteBridgePrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "refract" => {
                    self.handle_refract(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                _ => {
                    // Unknown frequency
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    return Ok(true);
                }
            }
        }
        
        // Ignore other pulse types
        Ok(false)
    }
}

// Export a function to create a new instance
// This will be used by the dynamic loading system
#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    match RemoteBridgePrism::new() {
        Ok(prism) => Box::new(prism),
        Err(e) => {
            eprintln!("Failed to create remote bridge prism: {}", e);
            // Return a placeholder that will fail during initialization
            struct FailingPrism;
            impl UVPrism for FailingPrism {
                fn init(&mut self, _spectrum: UVSpectrum) -> Result<()> {
                    Err(UVError::ExecutionError("Failed to initialize remote bridge prism".into()))
                }
                
                fn handle_pulse(&self, _id: Uuid, _pulse: &UVPulse, _link: &UVLink) -> Result<bool> {
                    Err(UVError::ExecutionError("Remote bridge prism not properly initialized".into()))
                }
            }
            
            Box::new(FailingPrism)
        }
    }
}
