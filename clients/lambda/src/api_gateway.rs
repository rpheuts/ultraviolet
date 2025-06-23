//! API Gateway WebSocket integration for Lambda
//!
//! This module provides utilities for sending messages back to WebSocket
//! clients through AWS API Gateway's management API.

use aws_config::BehaviorVersion;
use aws_sdk_apigatewaymanagement::{Client, primitives::Blob};
use tracing::{error, debug};

/// Helper for sending messages to WebSocket clients via API Gateway
pub struct ApiGatewaySender {
    client: Client,
    connection_id: String,
}

impl ApiGatewaySender {
    /// Create a new API Gateway sender
    pub async fn new(endpoint: String, connection_id: String) -> Self {
        debug!("Creating API Gateway sender for endpoint: {}, connection: {}", endpoint, connection_id);
        
        // Create API Gateway Management API client
        let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
        let client = Client::new(&config);
        
        Self {
            client,
            connection_id,
        }
    }
    
    /// Send a message to the WebSocket client
    pub async fn send_message(&self, data: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Sending message to client {}: {}", self.connection_id, data);
        
        match self.client
            .post_to_connection()
            .connection_id(&self.connection_id)
            .data(Blob::new(data.as_bytes()))
            .send()
            .await
        {
            Ok(_) => {
                debug!("Message sent successfully to client {}", self.connection_id);
                Ok(())
            },
            Err(e) => {
                error!("Failed to send message to client {}: {}", self.connection_id, e);
                Err(Box::new(e))
            }
        }
    }
}
