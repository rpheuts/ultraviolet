//! DateTime prism implementation for the Ultraviolet system.
//!
//! This crate provides a datetime prism that returns current date and time
//! information in multiple formats for AI agents and workflows.

use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// DateTime prism for getting current date and time information
pub struct DateTimePrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
}

impl DateTimePrism {
    /// Create a new DateTime prism
    pub fn new() -> Self {
        Self {
            spectrum: None,
        }
    }

    /// Handle 'now' frequency - get current date and time
    fn handle_now(&self, id: Uuid, link: &UVLink) -> Result<()> {
        let now: DateTime<Utc> = Utc::now();
        
        let response = json!({
            "timestamp": now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "unix_timestamp": now.timestamp(),
            "unix_timestamp_ms": now.timestamp_millis(),
            "date": now.format("%Y-%m-%d").to_string(),
            "time": now.format("%H:%M:%S").to_string(),
            "datetime": now.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        });

        link.emit_photon(id, response)?;
        link.emit_trap(id, None)?;
        Ok(())
    }
}

impl UVPrism for DateTimePrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }

    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "now" => {
                    self.handle_now(id, link)?;
                    return Ok(true);
                },
                _ => {
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(DateTimePrism::new())
}