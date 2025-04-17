//! ASR extractor implementation.
//!
//! This module provides functionality to extract application data from the ASR service.

use serde_json::{json, Value};
use uv_core::{PrismMultiplexer, Result, UVError, UVSpectrum};
use crate::writer::ExtractorWriter;
use crate::spectrum::HttpResponse;
use crate::extractors::Extractor;

/// ASR extractor implementation.
pub struct AsrExtractor<'a> {
    multiplexer: &'a PrismMultiplexer,
    spectrum: &'a UVSpectrum,
}

impl<'a> AsrExtractor<'a> {
    /// Create a new ASR extractor.
    pub fn new(multiplexer: &'a PrismMultiplexer, spectrum: &'a UVSpectrum) -> Self {
        Self {
            multiplexer,
            spectrum,
        }
    }
}

impl<'a> Extractor for AsrExtractor<'a> {
    fn process_user<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value> {
        writer.write_progress(&format!("Processing ASR applications for user: {}", user), Some("asr"), Some(user))?;

        // Create request body
        let request_body = json!({
            "securityStatus": ["REVIEW_IN_PROGRESS", "REVIEW_NOT_STARTED"],
            "orgLeaders": [user],
            "active": true,
            "personalApp": false,
            "pageSize": 1000,
            "from": 0,
            "sortFields": [{"name": "names", "direction": "asc"}]
        });

        // Make POST request to ASR service
        writer.write_progress("Fetching ASR applications...", Some("asr"), Some(user))?;
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum,
            json!({
                "url": "https://proxy.asr.security.amazon.dev/listApplications",
                "headers": {
                    "referer": "https://asr.security.amazon.dev",
                    "content-type": "application/json"
                },
                "body": request_body.to_string()
            }),
        )?;

        // Parse ASR response
        let data: Value = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse ASR response: {}", e)))?;

        writer.write_progress("ASR data processing complete", Some("asr"), Some(user))?;
        
        // Return the entire response data
        Ok(data)
    }
}
