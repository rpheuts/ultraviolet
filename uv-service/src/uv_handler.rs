use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::{stream::SplitSink, SinkExt};
use tokio::sync::Mutex;
use uuid::Uuid;
use uv_core::{PrismMultiplexer, Trap, UVError, UVLink, UVPulse, Wavefront};

use tracing::{error, debug};

use crate::{Result, ServiceError};

pub struct UVHandler {
    multiplexer: PrismMultiplexer
}

impl UVHandler {
    pub fn new() -> Result<Self> {
        Ok(Self {
            multiplexer: PrismMultiplexer::new()
        })
    }

    pub async fn process_wavefront(&self, wavefront: Wavefront, sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>) {
        // Get prism and frequency from wavefront
        let prism_id = wavefront.prism.clone();
        let frequency = wavefront.frequency.clone();
        
        debug!("Routing to prism: {}, frequency: {}", prism_id, frequency);
    
        // Process the wavefront
        match self.multiplexer.establish_link(&prism_id) {
            Ok(link) => {
                // Send the wavefront
                if let Err(e) = link.send_wavefront(wavefront.id, &prism_id, &frequency, wavefront.input.clone()) {
                    error!("Error sending wavefront: {}", e);
                    
                    // Create a trap for error
                    let error_trap = UVPulse::Trap(Trap {
                        id: wavefront.id,
                        error: Some(UVError::Other(format!("Failed to send wavefront: {}", e))),
                    });
                    
                    // Send error back to client
                    let error_msg = serde_json::to_string(&error_trap)
                        .unwrap_or_else(|_| r#"{"error":"Failed to serialize error"}"#.to_string());
                    
                    let mut sender_lock = sender.lock().await;
                    if let Err(e) = sender_lock.send(Message::Text(error_msg)).await {
                        error!("Failed to send error response: {}", e);
                    }
                    return;
                }
                
                // Process responses directly to keep link alive
                // until all responses are received
                if let Err(e) = self.process_responses(&link, wavefront.id, &sender).await {
                    error!("Error processing responses: {}", e);
                }
            },
            Err(e) => {
                error!("Error establishing link to prism: {}", e);
                        
                // Create a trap for error
                let error_trap = UVPulse::Trap(Trap {
                    id: wavefront.id,
                    error: Some(UVError::Other(format!("Failed to establish link to prism: {}", e))),
                });
                
                // Send error back to client
                let error_msg = serde_json::to_string(&error_trap)
                    .unwrap_or_else(|_| r#"{"error":"Failed to serialize error"}"#.to_string());
                
                let mut sender_lock = sender.lock().await;
                if let Err(e) = sender_lock.send(Message::Text(error_msg)).await {
                    error!("Failed to send error response: {}", e);
                }
            }
        }
    }
    
    /// Process responses from a prism directly
    pub async fn process_responses(
        &self,
        link: &UVLink, 
        wavefront_id: Uuid,
        sender: &Arc<Mutex<futures::stream::SplitSink<WebSocket, Message>>>,
    ) -> Result<()> {
        loop {
            // Get the next response from the prism
            match link.receive() {
                Ok(Some((_id, pulse))) => {
                    // Serialize the pulse
                    if let Ok(json) = serde_json::to_string(&pulse) {
                        // Forward to the WebSocket
                        let mut sender_lock = sender.lock().await;
                        if let Err(e) = sender_lock.send(Message::Text(json)).await {
                            error!("Failed to send response to WebSocket: {}", e);
                            return Err(ServiceError::WebSocketError(format!("Failed to send response: {}", e)));
                        }
                        
                        // If this is a trap, we're done with this request
                        if matches!(pulse, UVPulse::Trap(_)) {
                            debug!("Received trap, ending response processing");
                            return Ok(());
                        }
                    } else {
                        error!("Failed to serialize pulse");
                        return Err(ServiceError::SerializationError(serde_json::Error::io(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Failed to serialize response"
                        ))));
                    }
                },
                Ok(None) => {
                    // No message received, wait a bit
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                },
                Err(e) => {
                    error!("Error receiving from prism: {}", e);
                    
                    // Create a trap for the error
                    let error_trap = UVPulse::Trap(Trap {
                        id: wavefront_id,
                        error: Some(UVError::Other(format!("Prism error: {}", e))),
                    });
                    
                    // Send error to client
                    let error_msg = serde_json::to_string(&error_trap)
                        .unwrap_or_else(|_| r#"{"error":"Failed to serialize error"}"#.to_string());
                    
                    let mut sender_lock = sender.lock().await;
                    let _ = sender_lock.send(Message::Text(error_msg)).await;
                    return Err(ServiceError::PrismError(UVError::Other(format!("Error receiving from prism: {}", e))));
                }
            }
        }
    }    
}

