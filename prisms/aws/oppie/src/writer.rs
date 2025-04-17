//! Writer implementation for the oppie prism.
//!
//! This module provides a trait for writing progress messages and an implementation
//! that emits photons for streaming output.

use uuid::Uuid;
use uv_core::{UVLink, Result};
use crate::spectrum::ProgressMessage;

/// Trait for writing progress messages during extraction and upload.
pub trait ExtractorWriter {
    /// Write a line of text as a progress message.
    fn write_line(&mut self, line: &str) -> Result<()>;
    
    /// Write a progress message with service and user context.
    fn write_progress(&mut self, message: &str, service: Option<&str>, user: Option<&str>) -> Result<()>;
    
    /// Write a progress message with percentage.
    fn write_progress_percent(&mut self, message: &str, service: Option<&str>, user: Option<&str>, percent: f64) -> Result<()>;
}

/// Writer implementation that emits photons through a UVLink.
pub struct LinkWriter<'a> {
    /// The link to emit photons through
    link: &'a UVLink,
    
    /// The request ID to use for photons
    id: Uuid,
}

impl<'a> LinkWriter<'a> {
    /// Create a new LinkWriter.
    pub fn new(link: &'a UVLink, id: Uuid) -> Self {
        Self { link, id }
    }
}

impl<'a> ExtractorWriter for LinkWriter<'a> {
    fn write_line(&mut self, line: &str) -> Result<()> {
        let message = ProgressMessage {
            message: format!("{}\n", line),
            service: None,
            user: None,
            progress: None,
        };
        
        self.link.emit_photon(self.id, serde_json::to_value(message)?)
    }
    
    fn write_progress(&mut self, message: &str, service: Option<&str>, user: Option<&str>) -> Result<()> {
        let message = ProgressMessage {
            message: format!("{}\n", message),
            service: service.map(|s| s.to_string()),
            user: user.map(|u| u.to_string()),
            progress: None,
        };
        
        self.link.emit_photon(self.id, serde_json::to_value(message)?)
    }
    
    fn write_progress_percent(&mut self, message: &str, service: Option<&str>, user: Option<&str>, percent: f64) -> Result<()> {
        let message = ProgressMessage {
            message: format!("{}\n", message),
            service: service.map(|s| s.to_string()),
            user: user.map(|u| u.to_string()),
            progress: Some(percent),
        };
        
        self.link.emit_photon(self.id, serde_json::to_value(message)?)
    }
}
