//! Transport abstraction for the Ultraviolet system.
//!
//! The transport layer provides a low-level abstraction for sending and receiving
//! pulses between system components. It is used by the UVLink to handle
//! communication between prisms.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use crossbeam_channel::{Sender, Receiver, unbounded, RecvTimeoutError};

use crate::error::{UVError, Result};
use crate::pulse::UVPulse;

/// Transport abstraction for sending and receiving pulses.
pub trait Transport: Send + Sync {
    /// Send a pulse over the transport.
    fn send(&self, pulse: UVPulse) -> Result<()>;
    
    /// Receive a pulse from the transport with timeout.
    fn receive(&self) -> Result<Option<UVPulse>>;
    
    /// Close the transport.
    fn close(&self) -> Result<()>;
}

/// Create a pair of connected transports.
///
/// This function creates a pair of transports that are connected to each other.
/// Pulses sent on one transport can be received on the other.
pub fn create_transport_pair() -> (Box<dyn Transport>, Box<dyn Transport>) {
    // Create two pairs of channels
    let (tx1, rx2) = unbounded();
    let (tx2, rx1) = unbounded();
    
    // Create two transports with the channels
    let transport1 = ChannelTransport::new(tx1, rx1);
    let transport2 = ChannelTransport::new(tx2, rx2);
    
    (Box::new(transport1), Box::new(transport2))
}

/// Channel-based transport implementation.
pub struct ChannelTransport {
    sender: Sender<UVPulse>,
    receiver: Receiver<UVPulse>,
    closed: AtomicBool,
}

impl ChannelTransport {
    /// Create a new channel transport with the given sender and receiver.
    pub fn new(sender: Sender<UVPulse>, receiver: Receiver<UVPulse>) -> Self {
        Self {
            sender,
            receiver,
            closed: AtomicBool::new(false),
        }
    }
}

impl Transport for ChannelTransport {
    fn send(&self, pulse: UVPulse) -> Result<()> {
        // Check if the transport is closed
        if self.closed.load(Ordering::Relaxed) {
            return Err(UVError::TransportError("Transport is closed".to_string()));
        }
        
        // Send the pulse over the channel
        self.sender.send(pulse)
            .map_err(|_| UVError::TransportError("Failed to send data".to_string()))?;
        
        Ok(())
    }
    
    fn receive(&self) -> Result<Option<UVPulse>> {
        // Check if the transport is closed
        if self.closed.load(Ordering::Relaxed) {
            return Err(UVError::TransportError("Transport is closed".to_string()));
        }
        
        // Try to receive from the channel with a timeout
        match self.receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(pulse) => Ok(Some(pulse)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                Err(UVError::TransportError("Channel disconnected".to_string()))
            }
        }
    }
    
    fn close(&self) -> Result<()> {
        // Mark the transport as closed
        self.closed.store(true, Ordering::Relaxed);
        Ok(())
    }
}
