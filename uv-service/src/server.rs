//! WebSocket server implementation for the UV Service.
//!
//! This module handles incoming WebSocket connections and routes messages
//! to the appropriate prism handlers.

use std::sync::Arc;
use axum::{extract::ws::{Message, WebSocket, WebSocketUpgrade}, routing::get, Router};
use futures::StreamExt;
use tokio::sync::Mutex;
use tracing::{info, error, debug, warn};
use tower_http::services::{ServeDir, ServeFile};

use crate::{request_handler::UVRequestHandler, LogLevel, Result, ServiceError, ServiceOptions};

/// Shared application state
pub struct UVServer {
    /// Service configuration options
    handler: UVRequestHandler
}

impl UVServer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            handler: UVRequestHandler::new()?
        })
    }

    /// Run the WebSocket server with the provided options.
    pub async fn run_server(options: ServiceOptions) -> Result<()> {
        let server = Arc::new(UVServer::new()?);

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
        
        // Build the router
        let mut app = Router::new()
            // WebSocket handler
            .route("/ws", get(async move |ws: WebSocketUpgrade| {ws.on_upgrade(async move |socket| server.handle_socket(socket).await)}));
        
        // Add static file serving if enabled
        if options.serve_static {
            if let Some(static_dir) = &options.static_dir {
                info!("Serving static files from: {}", static_dir.display());
                
                // Serve static files at the root path
                app = app.nest_service(
                    "/",
                    ServeDir::new(static_dir)
                        .fallback(ServeFile::new(
                            static_dir.join("index.html")
                        ))
                );
            } else {
                warn!("Static file serving enabled but no directory specified");
            }
        }
        
        // Start the server
        info!("Starting UV Service on {}", options.bind_address);
        let listener = tokio::net::TcpListener::bind(options.bind_address).await
            .map_err(|e| ServiceError::ServerError(format!("Failed to bind to address: {}", e)))?;
            
        axum::serve(listener, app)
            .await
            .map_err(|e| ServiceError::ServerError(format!("Server error: {}", e)))?;
        
        Ok(())
    }

    /// Handle an established WebSocket connection
    pub async fn handle_socket(&self, socket: WebSocket) {
        let (sender, mut receiver) = socket.split();

        // Wrap the sender in Arc<Mutex<>> so it can be shared
        let sender = Arc::new(Mutex::new(sender));
        debug!("WebSocket connection established");
        
        // Process incoming messages
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received message: {}", text);
                    
                    self.handler.process_request(text, &sender).await;
                },
                Ok(Message::Close(_)) => {
                    debug!("WebSocket connection closed by client");
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
}
