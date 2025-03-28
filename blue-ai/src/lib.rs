use axum_server::bind;
use blue_core::prelude::*;
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tempfile::tempdir;

// Import our new modules
mod server;
mod ws;

pub struct AiModule {
    manifest: ModuleManifest,
    context: ModuleContext,
}

impl AiModule {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;

        let lib_path = home.join(".blue/modules");
        let manifest = ModuleManifest::load(lib_path.join("blue").join("ai").join("manifest.toml"))?;
        let context = ModuleContext::new(lib_path);

        Ok(Self { manifest, context })
    }
    
    // Format a terminal hyperlink - works in many modern terminals
    fn format_clickable_url(url: &str) -> String {
        format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", url, url)
    }
    
    // Start the web server
    fn start_web_server(&self, host: &str, port: u16, mut stdout: Option<&mut dyn Write>) -> Result<()> {
        // Create Tokio runtime
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Module(format!("Failed to create runtime: {}", e)))?;
        
        // Create a temp directory for file uploads
        let temp_dir = tempdir()
            .map_err(|e| Error::Module(format!("Failed to create temp directory: {}", e)))?
            .into_path();
            
        // Create a new context (as mentioned, we can create this ourselves)
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;
        let lib_path = home.join(".blue/modules");
        let context = ModuleContext::new(lib_path.clone());
        
        // Create a flag to track server running state
        let shutdown_flag = std::sync::Arc::new(AtomicBool::new(false));
        let shutdown_flag_ctrlc = shutdown_flag.clone();
        
        // Create shared state
        let state = std::sync::Arc::new(server::AppState {
            context: std::sync::Mutex::new(context),
            temp_dir: temp_dir.clone(),
            shutdown_flag: shutdown_flag.clone(),
        });
        
        // Handle Ctrl+C to gracefully shutdown
        let _ = ctrlc::set_handler(move || {
            shutdown_flag_ctrlc.store(true, Ordering::SeqCst);
        });
        
        // Create a flag for shutdown notification
        let should_log_shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let should_log_shutdown_clone = should_log_shutdown.clone();
        
        // Run server in the runtime
        rt.block_on(async move {
            // Build application router
            let app = axum::Router::new()
                // Static file routes
                .route("/", axum::routing::get(server::serve_index))
                .route("/assets/*path", axum::routing::get(server::serve_asset))
                .route("/:file", axum::routing::get(server::serve_static))
                
                // API routes
                .route("/api/chat", axum::routing::post(server::chat_endpoint))
                .route("/api/upload", axum::routing::post(server::upload_endpoint))
                .route("/api/shutdown", axum::routing::post(server::shutdown_endpoint))
                .route("/ws", axum::routing::get(server::websocket_handler))
                
                // CORS setup
                .layer(tower_http::cors::CorsLayer::permissive())
                
                // Add shared state
                .with_state(state.clone());
                
            // Bind address
            let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse()
                .map_err(|e| Error::Module(format!("Invalid address: {}", e)))?;
                
            // Start the server in a task (returns a handle we can abort)
            let server_handle = tokio::spawn(async move {
                if let Err(e) = bind(addr).serve(app.into_make_service()).await {
                    eprintln!("Server error: {}", e);
                }
            });
            
            // Wait for shutdown signal
            while !shutdown_flag.load(Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            
            // Set the shutdown flag
            should_log_shutdown_clone.store(true, Ordering::SeqCst);
            
            // Cancel the server task (this will gracefully shut down axum)
            server_handle.abort();
            
            Ok(()) as Result<()>
        })?;
        
        // Print shutdown message if needed
        if should_log_shutdown.load(Ordering::SeqCst) {
            if let Some(out) = stdout.as_mut() {
                let _ = writeln!(out, "\n✅ \x1B[1;32mServer successfully shut down\x1B[0m");
            }
        }
        
        Ok(())
    }
}

impl Module for AiModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Validate method exists
        if self.manifest.find_method(path).is_none() {
            return Err(Error::MethodNotFound(path.join(" ")));
        }

        match path {
            ["web"] => {
                // Extract port and host from args
                let args = args.as_object()
                    .ok_or_else(|| Error::Module("Invalid arguments".into()))?;
                
                let port = args.get("port")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(3000) as u16;
                    
                let host = args.get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("127.0.0.1");

                // Format clickable URL
                let url = format!("http://{}:{}", host, port);
                let clickable_url = Self::format_clickable_url(&url);

                let _ = println!("\n🚀 \x1B[1;36mBlue AI Web Server\x1B[0m");
                let _ = println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                let _ = println!("📊 \x1B[1;32mStatus:\x1B[0m Running");
                let _ = println!("🌐 \x1B[1;34mURL:\x1B[0m {}", clickable_url);
                let _ = println!("🛑 \x1B[1;33mStop:\x1B[0m Press Ctrl+C or use shutdown button");
                let _ = println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                
                // Start web server (blocking call)
                self.start_web_server(host, port, stdout)?;
                
                Ok(json!({"status": "shutdown"}))
            },
            _ => {
                // Add stream=true to args for regular calls
                let mut args_map = args.as_object()
                    .ok_or_else(|| Error::Module("Invalid arguments".into()))?
                    .clone();
                args_map.insert("stream".to_string(), json!(true));

                // Call core-ai module
                self.context.call_module("blue:core-ai", &["ask"], Value::Object(args_map), stdout, stderr)
            }
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(AiModule::new().expect("Failed to create ai module"))
}
