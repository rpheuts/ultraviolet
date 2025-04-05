use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::{stream::SplitSink, SinkExt};
use tokio::sync::Mutex;
use uv_core::{Trap, UVError, UVPulse};

use tracing::error;

use crate::{uv_handler::UVHandler, Result};

pub struct UVRequestHandler {
    handler: UVHandler
}

impl UVRequestHandler {
    pub fn new() -> Result<Self> {
        Ok(Self{
            handler: UVHandler::new()?
        })
    }

    pub async fn process_request(&self, text: String, sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>) {
        // Parse the message as a UVPulse
        match serde_json::from_str::<UVPulse>(&text) {
            Ok(pulse) => {
                // Process the pulse inline to avoid passing the sender around
                match pulse {
                    UVPulse::Wavefront(wavefront) => {
                        self.handler.process_wavefront(wavefront, sender).await;
                    },
                    _ => {
                        error!("Expected Wavefront message");
                        
                        // Create a trap for error
                        let error_trap = UVPulse::Trap(Trap {
                            id: uuid::Uuid::nil(),
                            error: Some(UVError::Other("Expected Wavefront message".to_string())),
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
            },
            Err(e) => {
                error!("Failed to parse message as UVPulse: {}", e);
                
                // Send error back to client
                let error_msg = format!(r#"{{"error":"Invalid pulse format: {}"}}"#, e);
                
                let mut sender_lock = sender.lock().await;
                if let Err(e) = sender_lock.send(Message::Text(error_msg)).await {
                    error!("Failed to send error response: {}", e);
                }
            }
        }
    }
}