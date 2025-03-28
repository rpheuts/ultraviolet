use std::io::Write;
use tokio::sync::mpsc;

/// A custom writer that forwards written data to a WebSocket sender channel
pub struct WebSocketWriter {
    sender: mpsc::UnboundedSender<String>,
}

impl WebSocketWriter {
    /// Create a new WebSocketWriter with the given sender channel
    pub fn new(sender: mpsc::UnboundedSender<String>) -> Self {
        Self { sender }
    }
}

impl Write for WebSocketWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Try to convert the buffer to a UTF-8 string
        if let Ok(s) = std::str::from_utf8(buf) {
            // Only send if non-empty
            if !s.is_empty() {
                // Use send on the unbounded channel
                if let Err(e) = self.sender.send(s.to_owned()) {
                    // Channel closed - this is a real error
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe, 
                        format!("WebSocket channel closed: {}", e)
                    ));
                }
            }
        } else {
            // Could not parse UTF-8 - this should be rare
            eprintln!("Warning: WebSocketWriter received invalid UTF-8 data");
        }
        
        // Return the buffer length to indicate all bytes were processed
        Ok(buf.len())
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        // No need to do anything for flush
        Ok(())
    }
}
