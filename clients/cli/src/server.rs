use std::{net::SocketAddr, path::PathBuf};
use anyhow::Result;
use uv_core::UVError;
use uv_service::{start_service, ServiceOptions};

pub async fn handle_server(bind_address: SocketAddr, server_static: bool, debug: bool) -> Result<()> {
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
        serve_static: server_static,
        static_dir: Some(get_web_dir()?),
        init_tracing: false,
        log_level,
    };

    open::that(format!("http://{}", options.bind_address))?;
    start_service(options).await.map_err(|e| anyhow::anyhow!("Service error: {}", e))?;

    Ok(())
}

fn get_web_dir() -> Result<PathBuf, UVError> {
    let home_dir = std::env::var("HOME").map_err(|_| UVError::Other("HOME environment variable not set".to_string()))?;
    let install_dir = std::env::var("UV_WEB_DIR").unwrap_or(format!("{}/.uv/assets/web", home_dir));
    Ok(PathBuf::from(install_dir))
}