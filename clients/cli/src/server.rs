use std::net::SocketAddr;
use anyhow::Result;
use uv_service::{start_service, ServiceOptions};

pub async fn handle_server(bind_address: SocketAddr, debug: bool) -> Result<()> {
    let log_level = if debug {
        uv_service::LogLevel::Debug
    } else {
        uv_service::LogLevel::Normal
    };

    let options = ServiceOptions {
        bind_address: bind_address,
        enable_tls: false,
        cert_path: None,
        key_path: None,
        serve_static: false,
        static_dir: None,
        init_tracing: false, // Don't initialize tracing again (already done in CLI)
        log_level,
    };

    start_service(options).await.map_err(|e| anyhow::anyhow!("Service error: {}", e))?;

    Ok(())
}