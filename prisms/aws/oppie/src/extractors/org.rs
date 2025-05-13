//! Org extractor implementation.
//!
//! This module provides functionality to extract organization data from Phonetool.
//! It builds a hierarchical organization chart for the specified users.

use serde_json::{json, Value};
use uv_core::{PrismMultiplexer, Result, UVError, UVSpectrum};
use crate::writer::ExtractorWriter;
use crate::spectrum::HttpResponse;
use crate::extractors::Extractor;

/// Org extractor implementation.
pub struct OrgExtractor<'a> {
    multiplexer: &'a PrismMultiplexer,
    spectrum: &'a UVSpectrum,
}

impl<'a> OrgExtractor<'a> {
    /// Create a new Org extractor.
    pub fn new(multiplexer: &'a PrismMultiplexer, spectrum: &'a UVSpectrum) -> Self {
        Self {
            multiplexer,
            spectrum,
        }
    }
    
    /// Process multiple users at once.
    pub fn process_user<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value> {
        // Process first user
        writer.write_progress(&format!("Processing user: {}", user), Some("org"), Some(&user))?;

        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://phonetool.amazon.com/users/{}/setup_org_chart.json", user),
                "headers": {
                    "referer": "https://phonetool.amazon.com/",
                    "accept": "*/*"
                }
            }),
        )?;

        // Parse JSON response
        let data: Value = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse org chart response: {}", e)))?;

        let mut result = None;

        // Find level 2 item for user
        if let Some(results) = data.get("results").and_then(|v| v.as_array()) {
            for item in results {
                if let (Some(level), Some(user)) = (
                    item.get("level").and_then(|v| v.as_i64()),
                    item.get("user")
                ) {
                    if level == 2 {
                        let mut user_data = user.clone();
                        if let Some(obj) = user_data.as_object_mut() {
                            obj.insert("reports".to_string(), json!([]));

                            // Process direct reports if any
                            if let Some(direct_reports) = user.get("direct_reports").and_then(|v| v.as_i64()) {
                                if direct_reports > 0 {
                                    if let Some(username) = user.get("login").and_then(|v| v.as_str()) {
                                        let reports = self.process_reports(username, writer)?;
                                        obj.insert("reports".to_string(), json!(reports));
                                    }
                                }
                            }
                        }
                        result = Some(user_data);
                        break;
                    }
                }
            }
        }

        let result = result.unwrap_or_else(|| json!({}));
        Ok(result)
    }
    
    /// Process reports recursively.
    fn process_reports<W: ExtractorWriter>(&self, username: &str, writer: &mut W) -> Result<Vec<Value>> {
        writer.write_progress(&format!("Processing reports for user: {}", username), Some("org"), Some(username))?;

        // Get org chart data
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://phonetool.amazon.com/users/{}/setup_org_chart.json", username),
                "headers": {
                    "referer": "https://phonetool.amazon.com/",
                    "accept": "*/*"
                }
            }),
        )?;

        // Parse JSON response
        let data: Value = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse org chart response: {}", e)))?;

        let mut reports = Vec::new();

        // Process level 3 items
        if let Some(results) = data.get("results").and_then(|v| v.as_array()) {
            for item in results {
                if let (Some(level), Some(user)) = (
                    item.get("level").and_then(|v| v.as_i64()),
                    item.get("user")
                ) {
                    if level == 3 {
                        let user_login = user.get("login").and_then(|v| v.as_str()).unwrap_or("unknown");
                        writer.write_progress(&format!("Processing user: {}", user_login), Some("org"), Some(username))?;

                        // Create user object with reports array
                        let mut user_data = user.clone();
                        if let Some(obj) = user_data.as_object_mut() {
                            obj.insert("reports".to_string(), json!([]));

                            // Process direct reports if any
                            if let Some(direct_reports) = user.get("direct_reports").and_then(|v| v.as_i64()) {
                                if direct_reports > 0 {
                                    if let Some(user_login) = user.get("login").and_then(|v| v.as_str()) {
                                        let sub_reports = self.process_reports(user_login, writer)?;
                                        obj.insert("reports".to_string(), json!(sub_reports));
                                    }
                                }
                            }
                        }
                        reports.push(user_data);
                    }
                }
            }
        }

        Ok(reports)
    }
}

impl<'a> Extractor for OrgExtractor<'a> {
    fn process_user<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value> {
        // Process a single user by wrapping it in a vector
        self.process_user(user, writer)
    }
}
