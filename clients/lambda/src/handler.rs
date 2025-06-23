//! WebSocket message handler for UV Lambda
//!
//! This module processes incoming WebSocket messages, routes them to the
//! appropriate prisms, and streams responses back to the client.

use serde_json::{json, Value};
use lambda_runtime::Error;
use tracing::{info, error, debug, warn};
use uv_core::{PrismMultiplexer, UVPulse, UVError, Trap};

use crate::api_gateway::ApiGatewaySender;

/// Handle an incoming WebSocket message
pub async fn handle_message(payload: Value) -> Result<Value, Error> {
    // Extract connection details from the Lambda event
    let connection_id = match payload["requestContext"]["connectionId"].as_str() {
        Some(id) => {
            debug!("Processing message for connection: {}", id);
            id
        },
        None => {
            error!("Missing connection ID in WebSocket event");
            return Ok(json!({"statusCode": 400, "body": "Missing connection ID"}));
        }
    };
    
    let domain = match payload["requestContext"]["domainName"].as_str() {
        Some(domain) => domain,
        None => {
            error!("Missing domain name in WebSocket event");
            return Ok(json!({"statusCode": 400, "body": "Missing domain name"}));
        }
    };
    
    let stage = match payload["requestContext"]["stage"].as_str() {
        Some(stage) => stage,
        None => {
            error!("Missing stage in WebSocket event");
            return Ok(json!({"statusCode": 400, "body": "Missing stage"}));
        }
    };
    
    let body = match payload["body"].as_str() {
        Some(body) => {
            debug!("Received message body: {}", body);
            body
        },
        None => {
            error!("Missing message body in WebSocket event");
            return Ok(json!({"statusCode": 400, "body": "Missing message body"}));
        }
    };
    
    // Parse the message as a UV pulse
    let pulse: UVPulse = match serde_json::from_str(body) {
        Ok(pulse) => {
            debug!("Successfully parsed UV pulse");
            pulse
        },
        Err(e) => {
            error!("Failed to parse message as UVPulse: {}", e);
            return Ok(json!({
                "statusCode": 400,
                "body": format!("Invalid pulse format: {}", e)
            }));
        }
    };
    
    // We only accept Wavefront messages from clients
    if let UVPulse::Wavefront(wavefront) = pulse {
        info!("Processing wavefront for prism: {}, frequency: {}", 
              wavefront.prism, wavefront.frequency);
        
        // Create API Gateway sender for responses
        let endpoint = format!("https://{}/{}", domain, stage);
        let sender = ApiGatewaySender::new(endpoint, connection_id.to_string()).await;
        
        // Process the wavefront
        match process_wavefront(wavefront, sender).await {
            Ok(_) => {
                info!("Successfully processed wavefront");
                Ok(json!({"statusCode": 200}))
            },
            Err(e) => {
                error!("Error processing wavefront: {}", e);
                Ok(json!({
                    "statusCode": 500,
                    "body": format!("Processing error: {}", e)
                }))
            }
        }
    } else {
        warn!("Received non-wavefront message from client");
        Ok(json!({
            "statusCode": 400,
            "body": "Expected a Wavefront message"
        }))
    }
}

/// Process a wavefront and handle all responses
async fn process_wavefront(
    wavefront: uv_core::Wavefront,
    sender: ApiGatewaySender,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize the prism multiplexer
    let multiplexer = PrismMultiplexer::new();
    
    // Establish link to the target prism
    let link = match multiplexer.establish_link(&wavefront.prism) {
        Ok(link) => {
            debug!("Successfully established link to prism: {}", wavefront.prism);
            link
        },
        Err(e) => {
            error!("Failed to establish link to prism {}: {}", wavefront.prism, e);
            
            // Send error trap to client
            let trap = UVPulse::Trap(Trap {
                id: wavefront.id,
                error: Some(UVError::Other(format!("Failed to establish link to prism: {}", e))),
            });
            
            if let Ok(json) = serde_json::to_string(&trap) {
                let _ = sender.send_message(json).await;
            }
            
            return Err(Box::new(e));
        }
    };
    
    // Send the wavefront to the prism
    if let Err(e) = link.send_wavefront(
        wavefront.id,
        &wavefront.prism,
        &wavefront.frequency,
        wavefront.input.clone(),
    ) {
        error!("Error sending wavefront to prism: {}", e);
        
        // Send error trap to client
        let trap = UVPulse::Trap(Trap {
            id: wavefront.id,
            error: Some(UVError::Other(format!("Failed to send wavefront: {}", e))),
        });
        
        if let Ok(json) = serde_json::to_string(&trap) {
            let _ = sender.send_message(json).await;
        }
        
        return Err(Box::new(e));
    }
    
    debug!("Wavefront sent successfully, waiting for responses");
    
    // Process all responses from the prism
    loop {
        match link.receive() {
            Ok(Some((_id, response_pulse))) => {
                debug!("Received response pulse: {:?}", response_pulse);
                
                // Serialize and send to client
                match serde_json::to_string(&response_pulse) {
                    Ok(json) => {
                        if let Err(e) = sender.send_message(json).await {
                            error!("Error sending response to client: {}", e);
                            // Continue processing other responses
                        }
                    },
                    Err(e) => {
                        error!("Error serializing response pulse: {}", e);
                        // Continue processing other responses
                    }
                }
                
                // If this is a trap, we're done processing
                if matches!(response_pulse, UVPulse::Trap(_)) {
                    info!("Received trap, ending response processing");
                    break;
                }
            },
            Ok(None) => {
                // No messages available yet, wait a bit
                debug!("No messages available, waiting...");
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            },
            Err(e) => {
                error!("Error receiving from prism link: {}", e);
                
                // Send error trap to client
                let trap = UVPulse::Trap(Trap {
                    id: wavefront.id,
                    error: Some(UVError::Other(format!("Link error: {}", e))),
                });
                
                if let Ok(json) = serde_json::to_string(&trap) {
                    let _ = sender.send_message(json).await;
                }
                
                return Err(Box::new(e));
            }
        }
    }
    
    debug!("Finished processing wavefront");
    Ok(())
}
