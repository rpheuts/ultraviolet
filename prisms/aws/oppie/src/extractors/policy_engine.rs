//! PolicyEngine extractor implementation.
//!
//! This module provides functionality to extract data from the PolicyEngine service.
//! It directly fetches CSV data for a user's risks.

use serde_json::{json, Value};
use uv_core::{PrismMultiplexer, Result, UVSpectrum};
use crate::writer::ExtractorWriter;
use crate::spectrum::HttpResponse;
use crate::extractors::Extractor;

/// PolicyEngine extractor implementation.
pub struct PolicyEngineExtractor<'a> {
    multiplexer: &'a PrismMultiplexer,
    spectrum: &'a UVSpectrum,
}

impl<'a> PolicyEngineExtractor<'a> {
    /// Create a new PolicyEngine extractor.
    pub fn new(multiplexer: &'a PrismMultiplexer, spectrum: &'a UVSpectrum) -> Self {
        Self {
            multiplexer,
            spectrum,
        }
    }
}

impl<'a> Extractor for PolicyEngineExtractor<'a> {
    fn process_user<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value> {
        // Call the internal implementation
        self.process_user_internal(user, writer)
    }
}

impl<'a> PolicyEngineExtractor<'a> {
    /// Process a user and extract PolicyEngine data (internal implementation).
    pub fn process_user_internal<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value> {
        writer.write_progress(&format!("Processing PolicyEngine risks for user: {}", user), Some("policy_engine"), Some(user))?;
        
        // Get policy engine data directly as CSV
        writer.write_progress("Fetching risks data as CSV...", Some("policy_engine"), Some(user))?;
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://policyengine.amazon.com/entities/{}.csv?front_page=approved&policy_class=all_policy_classes&policy_type=risks", user),
                "headers": {
                    "referer": "https://policyengine.amazon.com/",
                    "accept": "*/*"
                }
            }),
        )?;

        // Get CSV content from response body
        let csv = response.body.clone();
        
        writer.write_progress("PolicyEngine data processing complete", Some("policy_engine"), Some(user))?;
        
        // Return data with CSV
        Ok(json!({
            "csv": csv,
            "user": user
        }))
    }
}
