//! CTI extractor implementation.
//!
//! This module provides functionality to extract data from the CTI service.
//! It processes user groups and builds a mapping of owners to their groups.

use std::collections::HashMap;
use serde_json::{json, Value};
use uv_core::{PrismMultiplexer, Result, UVError, UVSpectrum};
use crate::writer::ExtractorWriter;
use crate::spectrum::HttpResponse;
use crate::extractors::Extractor;

/// CTI extractor implementation.
pub struct CtiExtractor<'a> {
    multiplexer: &'a PrismMultiplexer,
    spectrum: &'a UVSpectrum,
    owners: HashMap<String, Vec<String>>,
}

impl<'a> CtiExtractor<'a> {
    /// Create a new CTI extractor.
    pub fn new(multiplexer: &'a PrismMultiplexer, spectrum: &'a UVSpectrum) -> Self {
        Self {
            multiplexer,
            spectrum,
            owners: HashMap::new(),
        }
    }
}

impl<'a> Extractor for CtiExtractor<'a> {
    fn process_user<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value> {
        // Create a mutable copy of self to use the existing implementation
        let mut extractor = CtiExtractor {
            multiplexer: self.multiplexer,
            spectrum: self.spectrum,
            owners: HashMap::new(),
        };
        
        // Call the internal implementation
        extractor.process_user_internal(user, writer)
    }
}

impl<'a> CtiExtractor<'a> {
    /// Process a user and extract CTI data (internal implementation).
    pub fn process_user_internal<W: ExtractorWriter>(&mut self, user: &str, writer: &mut W) -> Result<Value> {
        writer.write_progress(&format!("Processing CTI groups for user: {}", user), Some("cti"), Some(user))?;
        
        // Get user's groups
        let groups = self.process_rollup(user, writer)?;

        // Process each group's owners
        for group in groups {
            self.process_owners(&group, writer)?;
        }

        // Return current state of owners map
        Ok(json!(self.owners))
    }
    
    /// Process rollup to get groups for a user.
    fn process_rollup<W: ExtractorWriter>(&mut self, user: &str, writer: &mut W) -> Result<Vec<String>> {
        writer.write_progress("Fetching user groups...", Some("cti"), Some(user))?;
        
        // Get groups for user
        let encoded_user = urlencoding::encode(user);
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://cti.amazon.com/rollup/{}/groups/", encoded_user),
                "headers": {
                    "referer": "https://cti.amazon.com/",
                    "accept": "*/*"
                }
            }),
        )?;

        // Parse JSON response
        let groups: Vec<String> = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse groups response: {}", e)))?;

        Ok(groups)
    }
    
    /// Process owners for a group.
    fn process_owners<W: ExtractorWriter>(&mut self, group: &str, writer: &mut W) -> Result<()> {
        writer.write_progress(&format!("Processing group: {}", group), Some("cti"), Some("group"))?;

        // Get group details
        let encoded_group = urlencoding::encode(group);
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://cti.amazon.com/group/{}/details", encoded_group),
                "headers": {
                    "referer": "https://cti.amazon.com/",
                    "accept": "*/*"
                }
            }),
        )?;

        // Parse JSON response
        let details: Value = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse group details response: {}", e)))?;

        // Get owners string and split into users
        let owners = details.get("owners")
            .and_then(|v| v.as_str())
            .ok_or_else(|| UVError::ExecutionError("Missing owners field in group details".to_string()))?;

        // Process each owner
        for user in owners.replace('\'', "").split(';') {
            if user.is_empty() {
                continue;
            }

            // Add group to user's list of groups
            self.owners.entry(user.to_string())
                .or_insert_with(Vec::new)
                .push(group.replace('#', ""));
        }

        Ok(())
    }
}
