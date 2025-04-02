//! Transport abstraction for the Ultraviolet system.
//!
//! The transport layer provides a low-level abstraction for sending and receiving
//! raw data between system components. It is used by the UVLink to handle
//! communication between prisms.

use async_trait::async_trait;
use tokio::sync::{mpsc::{self, Receiver, Sender}, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{UVError, Result};

/// Transport abstraction for sending and receiving raw data.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send raw data over the transport.
    async fn send(&self, data: Vec<u8>) -> Result<()>;
    
    /// Receive raw data from the transport.
    async fn receive(&self) -> Result<Option<Vec<u8>>>;
    
    /// Close the transport.
    async fn close(&self) -> Result<()>;
}

/// Create a pair of connected transports.
///
/// This function creates a pair of transports that are connected to each other.
/// Data sent on one transport can be received on the other.
pub fn create_transport_pair() -> (Box<dyn Transport>, Box<dyn Transport>) {
    // Create two pairs of channels
    let (tx1, rx2) = mpsc::channel(100);
    let (tx2, rx1) = mpsc::channel(100);
    
    // Create two transports with the channels
    let transport1 = InMemoryTransport::new(tx1, rx1);
    let transport2 = InMemoryTransport::new(tx2, rx2);
    
    (Box::new(transport1), Box::new(transport2))
}

/// In-memory transport implementation for testing and local communication.
///
/// This transport uses channels to send and receive data between two endpoints.
pub struct InMemoryTransport {
    sender: Sender<Vec<u8>>,
    receiver: Mutex<Option<Receiver<Vec<u8>>>>,
    closed: AtomicBool,
}

impl InMemoryTransport {
    /// Create a new in-memory transport with the given sender and receiver.
    pub fn new(sender: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>>) -> Self {
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
            closed: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl Transport for InMemoryTransport {
    async fn send(&self, data: Vec<u8>) -> Result<()> {
        // Check if the transport is closed
        if self.closed.load(Ordering::Relaxed) {
            return Err(UVError::TransportError("Transport is closed".to_string()));
        }
        
        // Send the data over the channel
        self.sender.send(data).await
            .map_err(|e| UVError::TransportError(format!("Failed to send data: {}", e)))?;
        
        Ok(())
    }
    
    async fn receive(&self) -> Result<Option<Vec<u8>>> {
        // Check if the transport is closed
        if self.closed.load(Ordering::Relaxed) {
            return Err(UVError::TransportError("Transport is closed".to_string()));
        }
        
        // Get the receiver
        let mut receiver_guard = self.receiver.lock().await;
        let receiver = match &mut *receiver_guard {
            Some(rx) => rx,
            None => return Ok(None),
        };
        
        // Receive data from the channel
        let result = match receiver.recv().await {
            Some(data) => Ok(Some(data)),
            None => {
                // The sender has been dropped, so we'll never receive more data
                *receiver_guard = None;
                Ok(None)
            }
        };
        
        // Drop the guard before returning
        drop(receiver_guard);
        
        result
    }
    
    async fn close(&self) -> Result<()> {
        // Mark the transport as closed
        self.closed.store(true, Ordering::Relaxed);
        
        // Clear the receiver
        let mut receiver_guard = self.receiver.lock().await;
        *receiver_guard = None;
        
        Ok(())
    }
}
