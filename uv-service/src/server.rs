//! WebSocket server implementation for the UV Service.
//!
//! This module handles incoming WebSocket connections and routes messages
//! to the appropriate prism handlers.

use std::sync::Arc;

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tracing::{info, error, debug};
use uuid::Uuid;
use serde_json;
use uv_core::{PrismMultiplexer, UVLink, UVPulse, Trap, UVError};

use crate::{ServiceOptions, Result, ServiceError, router::PulseRouter, LogLevel};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// The prism multiplexer for handling prism connections
    multiplexer: Arc<PrismMultiplexer>,
    
    /// The pulse router for handling message routing
    _router: Arc<PulseRouter>,
    
    /// Service configuration options
    _options: ServiceOptions,
}

/// Run the WebSocket server with the provided options.
pub async fn run_server(options: ServiceOptions) -> Result<()> {
    // Initialize tracing if requested, with appropriate log level
    if options.init_tracing {
        let filter = match options.log_level {
            LogLevel::Debug => "uv_service=debug,tower_http=debug",
            LogLevel::Normal => "uv_service=warn,tower_http=warn",
            LogLevel::Quiet => "uv_service=error,tower_http=error",
        };
        
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .init();
    }
    
    // Create the multiplexer
    let multiplexer = Arc::new(PrismMultiplexer::new());
    
    // Create the router
    let router = Arc::new(PulseRouter::new(multiplexer.clone()));
    
    // Create shared state
    let state = AppState {
        multiplexer,
        _router: router,
        _options: options.clone(),
    };
    
    // Build the router
    let app = Router::new()
        // WebSocket handler
        .route("/ws", get(websocket_handler))
        // Add state
        .with_state(state);
    
    // Start the server
    info!("Starting UV Service on {}", options.bind_address);
    let listener = tokio::net::TcpListener::bind(options.bind_address).await
        .map_err(|e| ServiceError::ServerError(format!("Failed to bind to address: {}", e)))?;
        
    axum::serve(listener, app)
        .await
        .map_err(|e| ServiceError::ServerError(format!("Server error: {}", e)))?;
    
    Ok(())
}

/// Handle WebSocket upgrade requests
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Session for managing WebSocket connections and associated resources
struct WebSocketSession {
    links: Vec<UVLink>,
}

impl Drop for WebSocketSession {
    fn drop(&mut self) {
        // Links are automatically dropped here, which triggers their UVLink::drop
        // implementation to send Extinguish signals to prisms
        info!("WebSocket session ended, dropping {} prism links", self.links.len());
    }
}

/// Handle an established WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (sender, mut receiver) = socket.split();
    
    // Wrap the sender in Arc<Mutex<>> so it can be shared
    let sender = Arc::new(Mutex::new(sender));
    
    // Create a session to manage prism links for this WebSocket connection
    let mut session = WebSocketSession { links: Vec::new() };
    
    info!("WebSocket connection established");
    
    // Process incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received message: {}", text);
                
                // Parse the message as a UVPulse
                match serde_json::from_str::<UVPulse>(&text) {
                    Ok(pulse) => {
                        // Process the pulse inline to avoid passing the sender around
                        match pulse {
                            UVPulse::Wavefront(wavefront) => {
                                // Get prism and frequency from wavefront
                                let prism_id = wavefront.prism.clone();
                                let frequency = wavefront.frequency.clone();
                                
                                debug!("Routing to prism: {}, frequency: {}", prism_id, frequency);
                                
                                // Process the wavefront
                                match state.multiplexer.establish_link(&prism_id) {
                                    Ok(link) => {
                                        // Add link to session for later cleanup
                                        session.links.push(link.clone());
                                        
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
                                            continue;
                                        }
                                        
                                        // Process responses directly to keep link alive
                                        // until all responses are received
                                        if let Err(e) = process_responses(&link, wavefront.id, &sender).await {
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
            },
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by client");
                break;
            },
            Ok(_) => {
                // Ignore other message types
            },
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }
    
    // The session will be dropped here, which will clean up all prism links
    info!("WebSocket connection terminated");
}

/// Process responses from a prism directly
async fn process_responses(
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
