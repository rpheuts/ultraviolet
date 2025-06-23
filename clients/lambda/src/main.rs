//! UV Lambda WebSocket handler
//!
//! This module implements a Lambda function that handles WebSocket messages
//! through AWS API Gateway and processes them using the UV prism system.

use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde_json::{json, Value};
use tracing::{info, error};

mod handler;
mod api_gateway;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing for Lambda CloudWatch logs
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_ansi(false) // Lambda doesn't support ANSI colors
        .without_time() // Lambda adds timestamps
        .init();

    info!("UV Lambda handler initializing");
    
    // Run Lambda service
    lambda_runtime::run(service_fn(function_handler)).await?;
    
    Ok(())
}

async fn function_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    info!("Received Lambda event");
    
    // Route based on WebSocket event type
    match event.payload["requestContext"]["routeKey"].as_str() {
        Some("$connect") => {
            info!("WebSocket connection established");
            Ok(json!({"statusCode": 200}))
        },
        Some("$disconnect") => {
            info!("WebSocket connection disconnected");
            Ok(json!({"statusCode": 200}))
        },
        Some("$default") => {
            info!("Received WebSocket message");
            handler::handle_message(event.payload).await
        },
        _ => {
            error!("Unknown route key in WebSocket event");
            Ok(json!({"statusCode": 400}))
        },
    }
}
