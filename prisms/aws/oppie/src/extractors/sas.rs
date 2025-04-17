//! SAS extractor implementation.
//!
//! This module provides functionality to extract data from the SAS service.

use serde_json::{json, Value};
use uv_core::{PrismMultiplexer, Result, UVError, UVSpectrum};
use crate::writer::ExtractorWriter;
use crate::spectrum::HttpResponse;
use crate::extractors::Extractor;

/// SAS extractor implementation.
pub struct SasExtractor<'a> {
    multiplexer: &'a PrismMultiplexer,
    spectrum: &'a UVSpectrum,
    items: Vec<Value>,
}

impl<'a> SasExtractor<'a> {
    /// Create a new SAS extractor.
    pub fn new(multiplexer: &'a PrismMultiplexer, spectrum: &'a UVSpectrum) -> Self {
        Self {
            multiplexer,
            spectrum,
            items: Vec::new(),
        }
    }
}

impl<'a> Extractor for SasExtractor<'a> {
    fn process_user<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value> {
        // Create a mutable copy of self to use the existing implementation
        let mut extractor = SasExtractor {
            multiplexer: self.multiplexer,
            spectrum: self.spectrum,
            items: Vec::new(),
        };
        
        // Call the internal implementation
        extractor.process_user_internal(user, writer)
    }
}

impl<'a> SasExtractor<'a> {
    
    /// Process a user and extract SAS data (internal implementation).
    pub fn process_user_internal<W: ExtractorWriter>(&mut self, user: &str, writer: &mut W) -> Result<Value> {
        writer.write_progress(&format!("Processing SAS data for user: {}", user), Some("sas"), Some(user))?;
        
        // Get user's risks
        writer.write_progress("Fetching risks data...", Some("sas"), Some(user))?;
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://sas.corp.amazon.com/api/users/{}/campaignTypes/NON_ADVISORY/risks", user),
                "headers": {
                    "referer": "https://sas.corp.amazon.com/"
                }
            }),
        )?;

        // Parse risks response
        let mut risks_data: Value = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse risks response: {}", e)))?;

        // Add owner login to data
        if let Some(obj) = risks_data.as_object_mut() {
            obj.insert("ownerLogin".to_string(), json!(user));
        }

        // Add risks data to items
        self.items.push(risks_data.clone());

        // Process reports recursively
        writer.write_progress("Processing reports...", Some("sas"), Some(user))?;
        self.process_reports(user, writer)?;

        writer.write_progress("SAS data processing complete", Some("sas"), Some(user))?;
        
        // Return all accumulated items
        Ok(json!(self.items))
    }

    /// Process reports recursively.
    fn process_reports<W: ExtractorWriter>(&mut self, user: &str, writer: &mut W) -> Result<()> {
        writer.write_progress(&format!("Processing reports for user: {}", user), Some("sas"), Some(user))?;

        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://sas.corp.amazon.com/api/users/{}/campaignTypes/NON_ADVISORY/summary/allReports", user),
                "headers": {
                    "referer": "https://sas.corp.amazon.com/"
                }
            }),
        )?;

        let data: Value = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse reports response: {}", e)))?;

        // Process direct reports if any
        if let Some(individual_list) = data.get("individualCountList").and_then(|v| v.as_array()) {
            let total_reports = individual_list.len();
            
            for (i, user_obj) in individual_list.iter().enumerate() {
                if let Some(owner_login) = user_obj.get("ownerLogin").and_then(|v| v.as_str()) {
                    if owner_login != user {  // Don't process the same user again
                        let progress = ((i + 1) as f64 / total_reports as f64) * 100.0;
                        writer.write_progress_percent(
                            &format!("Processing report {}/{}: {}", i + 1, total_reports, owner_login),
                            Some("sas"),
                            Some(user),
                            progress
                        )?;
                        
                        // Get risks for this report
                        let risks_response = self.multiplexer.refract_and_absorb::<HttpResponse>(
                            "curl.get",
                            self.spectrum,
                            json!({
                                "url": format!("https://sas.corp.amazon.com/api/users/{}/campaignTypes/NON_ADVISORY/risks", owner_login),
                                "headers": {
                                    "referer": "https://sas.corp.amazon.com/"
                                }
                            }),
                        )?;

                        let mut risks_data: Value = serde_json::from_str(&risks_response.body)
                            .map_err(|e| UVError::ExecutionError(format!("Failed to parse risks response: {}", e)))?;

                        // Add owner login to data
                        if let Some(obj) = risks_data.as_object_mut() {
                            obj.insert("ownerLogin".to_string(), json!(owner_login));
                        }

                        // Add to items
                        self.items.push(risks_data);

                        // Process their reports recursively (but limit depth to avoid excessive recursion)
                        // In a real implementation, we might want to limit the recursion depth
                        self.process_reports(owner_login, writer)?;
                    }
                }
            }
        }

        Ok(())
    }
}
