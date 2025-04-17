//! Shepherd extractor implementation.
//!
//! This module provides functionality to extract data from the Shepherd service.
//! It recursively processes issues for a user and their direct reports.

use serde_json::{json, Value};
use uv_core::{PrismMultiplexer, Result, UVError, UVSpectrum};
use crate::writer::ExtractorWriter;
use crate::spectrum::HttpResponse;
use crate::extractors::Extractor;

/// Shepherd extractor implementation.
pub struct ShepherdExtractor<'a> {
    multiplexer: &'a PrismMultiplexer,
    spectrum: &'a UVSpectrum,
    issues: Vec<Value>,
}

impl<'a> ShepherdExtractor<'a> {
    /// Create a new Shepherd extractor.
    pub fn new(multiplexer: &'a PrismMultiplexer, spectrum: &'a UVSpectrum) -> Self {
        Self {
            multiplexer,
            spectrum,
            issues: Vec::new(),
        }
    }
}

impl<'a> Extractor for ShepherdExtractor<'a> {
    fn process_user<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value> {
        // Create a mutable copy of self to use the existing implementation
        let mut extractor = ShepherdExtractor {
            multiplexer: self.multiplexer,
            spectrum: self.spectrum,
            issues: Vec::new(),
        };
        
        // Call the internal implementation
        extractor.process_user_internal(user, writer)
    }
}

impl<'a> ShepherdExtractor<'a> {
    /// Process a user and extract Shepherd data (internal implementation).
    pub fn process_user_internal<W: ExtractorWriter>(&mut self, user: &str, writer: &mut W) -> Result<Value> {
        // Clear any existing issues
        self.issues.clear();

        // Process user and their reports recursively
        self.process_user_recursive(user, writer)?;

        // Return all accumulated issues
        Ok(json!(self.issues))
    }
    
    /// Process a user and their direct reports recursively.
    fn process_user_recursive<W: ExtractorWriter>(&mut self, user: &str, writer: &mut W) -> Result<()> {
        writer.write_progress(&format!("Processing issues for user: {}", user), Some("shepherd"), Some(user))?;

        // Get employee info for direct reports
        writer.write_progress("Fetching employee info...", Some("shepherd"), Some(user))?;
        let employee_response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://shepherd.a2z.com/api/proxy/employees/{}", user),
                "headers": {
                    "referer": "https://shepherd.a2z.com/",
                    "accept": "*/*",
                    "priority": "u=1, i",
                    "x-impersonation-user": user
                }
            }),
        )?;

        // Parse employee response
        let employee: Value = serde_json::from_str(&employee_response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse employee response: {}", e)))?;

        // Get user's issues
        writer.write_progress("Fetching issues...", Some("shepherd"), Some(user))?;
        let issues_response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://shepherd.a2z.com/api/proxy/v2/issues?issueStates=%5B%22ASSIGNED%22%5D&owner=%7B%22reportingHierarchy%22%3A%5B%7B%22alias%22%3A%22{}%22%7D%5D%7D&pageSize=100&priority=TOP&severities=%5B%22CRITICAL%22%2C%22HIGH%22%2C%22MEDIUM%22%2C%22LOW%22%5D", user),
                "headers": {
                    "referer": "https://shepherd.a2z.com/",
                    "accept": "*/*",
                    "priority": "u=1, i",
                    "x-impersonation-user": user
                }
            }),
        )?;

        // Parse issues response
        let issues_data: Value = serde_json::from_str(&issues_response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse issues response: {}", e)))?;

        // Add issues from response
        if let Some(issues_list) = issues_data.get("issues").and_then(|v| v.as_array()) {
            writer.write_progress(
                &format!("Found {} issues for user {}", issues_list.len(), user),
                Some("shepherd"),
                Some(user)
            )?;
            self.issues.extend(issues_list.iter().cloned());
        }

        // Process direct reports recursively
        if let Some(reports) = employee.get("directReports").and_then(|v| v.as_array()) {
            let total_reports = reports.len();
            writer.write_progress(
                &format!("Processing {} direct reports for {}", total_reports, user),
                Some("shepherd"),
                Some(user)
            )?;
            
            for (i, report) in reports.iter().enumerate() {
                if let Some(alias) = report.get("alias").and_then(|v| v.as_str()) {
                    let progress = ((i + 1) as f64 / total_reports as f64) * 100.0;
                    writer.write_progress_percent(
                        &format!("Processing report {}/{}: {}", i + 1, total_reports, alias),
                        Some("shepherd"),
                        Some(user),
                        progress
                    )?;
                    
                    match self.process_user_recursive(alias, writer) {
                        Ok(_) => (),
                        Err(e) => {
                            writer.write_progress(
                                &format!("Failed to process issues for user '{}': {}", alias, e),
                                Some("shepherd"),
                                Some(user)
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
