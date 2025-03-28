use axum::{
    body::Body,
    extract::{
        Path,
        State,
        multipart::Multipart,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::{header, Response, StatusCode},
    response::IntoResponse,
    Json
};
use blue_core::prelude::*;
use futures::{sink::SinkExt, stream::StreamExt};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use uuid::Uuid;

// Include the web directory at compile time
static WEB_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/web");

// Shared application state
pub struct AppState {
    pub context: Mutex<ModuleContext>,
    pub temp_dir: PathBuf,
    pub shutdown_flag: Arc<AtomicBool>, // New shutdown flag
}

// Chat message request
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    #[serde(default)]
    pub files: Vec<String>,
}

// File upload response
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub file_paths: Vec<String>,
}

// WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

// Process WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Channel for sending messages to the WebSocket (unbounded to avoid message loss)
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    
    // Spawn a task to forward messages from the unbounded channel to WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sender.send(axum::extract::ws::Message::Text(message)).await.is_err() {
                // Error is not critical enough to log - client might have disconnected
                break;
            }
        }
    });
    
    // Process incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            match message {
                axum::extract::ws::Message::Text(text) => {
                    // Parse the incoming message
                    if let Ok(chat_request) = serde_json::from_str::<ChatRequest>(&text) {
                        // Process the request
                        let state_clone = state.clone();  // This extends the lifetime to 'static
                        let tx_clone = tx.clone();
                        
                        // Create shared channel for sending AI responses
                        let tx_clone_for_blocking = tx_clone.clone();
                        
                        // Spawn a task to handle the AI request
                        tokio::spawn(async move {
                            // Create args
                            let args = json!({
                                "prompt": chat_request.prompt,
                                "files": chat_request.files,
                                "stream": true
                            });
                            
                            // Use spawn_blocking for the synchronous call_module to avoid blocking the async runtime
                            let result = tokio::task::spawn_blocking(move || {
                                // Create a custom writer that forwards to WebSocket
                                let mut writer = crate::ws::WebSocketWriter::new(tx_clone_for_blocking);
                                
                                // Call core-ai with the writer (this is a synchronous operation)
                                let context_result = {
                                    let mut context = state_clone.context.lock().unwrap();
                                    context.call_module(
                                        "blue:core-ai", 
                                        &["ask"], 
                                        args, 
                                        Some(&mut writer),
                                        None
                                    )
                                };
                                
                                context_result
                            }).await;
                            
                            // Check for errors at both levels
                            match result {
                                Ok(context_result) => {
                                    if let Err(e) = context_result {
                                        let _ = tx_clone.send(format!("Error: {}", e));
                                    }
                                },
                                Err(join_error) => {
                                    let _ = tx_clone.send(format!("Task error: {}", join_error));
                                }
                            }
                            
                            // Signal completion
                            let _ = tx_clone.send("\n[DONE]".to_string());
                        });
                    }
                }
                axum::extract::ws::Message::Close(_) => break,
                _ => {}
            }
        }
    });
    
    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

// Handle file uploads
pub async fn upload_endpoint(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_paths = Vec::new();
    
    // Process each file in the multipart form
    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = match field.file_name() {
            Some(name) => name.to_string(),
            None => continue,
        };
        
        // Generate unique filename
        let uuid = Uuid::new_v4();
        let file_path = state.temp_dir.join(format!("{}-{}", uuid, file_name));
        
        // Convert to string path
        let path_str = file_path.to_string_lossy().to_string();
        
        // Read the file data
        let data = match field.bytes().await {
            Ok(data) => data,
            Err(_) => continue,
        };
        
        // Write to temp file
        if let Ok(_) = tokio::fs::write(&file_path, &data).await {
            file_paths.push(path_str);
        }
    }
    
    // Return list of saved files
    Json(UploadResponse { file_paths })
}

// Chat endpoint (for non-WebSocket requests)
pub async fn chat_endpoint(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> impl IntoResponse {
    // Create args
    let args = json!({
        "prompt": request.prompt,
        "files": request.files,
        "stream": false
    });
    
    // Call core-ai directly (non-streaming)
    let result = {
        let mut context = state.context.lock().unwrap();
        context.call_module("blue:core-ai", &["ask"], args, None, None)
    };
    
    match result {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    }
}

// Shutdown endpoint
pub async fn shutdown_endpoint(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Set the shutdown flag to trigger server termination
    state.shutdown_flag.store(true, Ordering::SeqCst);
    "Shutdown initiated".to_string()
}

// Serve static index.html
pub async fn serve_index() -> impl IntoResponse {
    match WEB_DIR.get_file("index.html") {
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(file.contents()))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap(),
    }
}

// Serve other static files
pub async fn serve_static(Path(file): Path<String>) -> impl IntoResponse {
    let mime_type = match file.split('.').last() {
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("html") => "text/html",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    };
    
    match WEB_DIR.get_file(&file) {
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime_type)
            .body(Body::from(file.contents()))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap(),
    }
}

// Serve assets
pub async fn serve_asset(Path(path): Path<String>) -> impl IntoResponse {
    let asset_path = format!("assets/{}", path);
    let mime_type = match path.split('.').last() {
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    };
    
    match WEB_DIR.get_file(&asset_path) {
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime_type)
            .body(Body::from(file.contents()))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap(),
    }
}
