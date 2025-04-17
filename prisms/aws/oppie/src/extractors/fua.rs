//! FUA extractor implementation.
//!
//! This module provides functionality to extract data from the Fleet User Association service.
//! It recursively processes fleets and their children, including score information.

use serde_json::{json, Value};
use uv_core::{PrismMultiplexer, Result, UVError, UVSpectrum};
use crate::writer::ExtractorWriter;
use crate::spectrum::HttpResponse;
use crate::extractors::Extractor;
use std::time::{SystemTime, UNIX_EPOCH};

/// FUA extractor implementation.
pub struct FuaExtractor<'a> {
    multiplexer: &'a PrismMultiplexer,
    spectrum: &'a UVSpectrum,
}

impl<'a> FuaExtractor<'a> {
    /// Create a new FUA extractor.
    pub fn new(multiplexer: &'a PrismMultiplexer, spectrum: &'a UVSpectrum) -> Self {
        Self {
            multiplexer,
            spectrum,
        }
    }
    
    /// Get fleet information.
    fn get_fleet_info(&self, fleet_id: &str) -> Result<Value> {
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://fleet-widget.corp.amazon.com/api/v1/fleets/{}", fleet_id),
                "headers": {
                    "referer": "https://fua.corp.amazon.com/",
                    "accept": "*/*"
                }
            }),
        )?;

        // Parse fleet info response
        serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse fleet info response: {}", e)))
    }
    
    /// Process fleet score information.
    fn process_fleet(&self, fleet_id: &str) -> Result<Value> {
        // Calculate timestamp (11 days ago)
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64 - (11 * 24 * 60 * 60); // 11 days in seconds

        let request_body = json!({
            "filters": {
                "specifiedFilters": {},
                "wildcardFilters": {}
            },
            "timestamp": timestamp,
            "csiPlan": false,
            "useShadowMode": true
        });

        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum,
            json!({
                "url": format!("https://fua.corp.amazon.com/fleet/{}/credit-score/overview", fleet_id),
                "headers": {
                    "referer": "https://fua.corp.amazon.com/",
                    "content-type": "application/json"
                },
                "body": request_body.to_string()
            }),
        )?;

        // Parse fleet score response
        serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse fleet score response: {}", e)))
    }
    
    /// Process children fleets recursively.
    fn process_children<W: ExtractorWriter>(&self, fleet_id: &str, writer: &mut W) -> Result<Vec<Value>> {
        let mut result = Vec::new();

        // Try to get children, but don't fail if we can't
        let response = match self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://fleet-widget.corp.amazon.com/api/v1/fleets/{}/children?include_deleted=false", fleet_id),
                "headers": {
                    "referer": "https://fua.corp.amazon.com/",
                    "accept": "*/*"
                }
            }),
        ) {
            Ok(response) => response,
            Err(e) => {
                writer.write_progress(
                    &format!("Failed to get children for fleet {}: {}", fleet_id, e),
                    Some("fua"),
                    Some(fleet_id)
                )?;
                return Ok(result);
            }
        };

        // Parse children response
        let children: Vec<Value> = match serde_json::from_str(&response.body) {
            Ok(children) => children,
            Err(e) => {
                writer.write_progress(
                    &format!("Failed to parse children response: {}", e),
                    Some("fua"),
                    Some(fleet_id)
                )?;
                return Ok(result);
            }
        };

        // Process each child
        for child in children {
            if let Some(id) = child.get("id").and_then(|v| v.as_str()) {
                writer.write_progress(&format!("Processing fleet: {}", id), Some("fua"), Some(id))?;

                // Create child object with score info
                let mut child_data = child.clone();
                if let Some(obj) = child_data.as_object_mut() {
                    match self.process_fleet(id) {
                        Ok(score_info) => {
                            obj.insert("scoreInfo".to_string(), score_info);
                        }
                        Err(e) => {
                            writer.write_progress(
                                &format!("Failed to get score for fleet {}: {}", id, e),
                                Some("fua"),
                                Some(id)
                            )?;
                        }
                    }
                }
                result.push(child_data);

                // Process child's children recursively
                match self.process_children(id, writer) {
                    Ok(mut children) => result.append(&mut children),
                    Err(e) => {
                        writer.write_progress(
                            &format!("Failed to process children for fleet {}: {}", id, e),
                            Some("fua"),
                            Some(id)
                        )?;
                    }
                }
            }
        }

        Ok(result)
    }
    
    /// Process a fleet ID.
    pub fn process_fleet_id<W: ExtractorWriter>(&self, fleet_id: &str, writer: &mut W) -> Result<Value> {
        writer.write_progress(&format!("Processing fleet: {}", fleet_id), Some("fua"), Some(fleet_id))?;

        // Get parent fleet info
        writer.write_progress("Fetching fleet info...", Some("fua"), Some(fleet_id))?;
        let mut parent_fleet = self.get_fleet_info(fleet_id)?;
        
        // Add score info to parent fleet
        writer.write_progress("Fetching fleet score...", Some("fua"), Some(fleet_id))?;
        if let Some(obj) = parent_fleet.as_object_mut() {
            match self.process_fleet(fleet_id) {
                Ok(score_info) => {
                    obj.insert("scoreInfo".to_string(), score_info);
                }
                Err(e) => {
                    writer.write_progress(
                        &format!("Failed to get score for fleet {}: {}", fleet_id, e),
                        Some("fua"),
                        Some(fleet_id)
                    )?;
                }
            }
        }

        // Create result array with parent fleet
        let mut result = vec![parent_fleet];

        // Process children recursively
        writer.write_progress("Processing child fleets...", Some("fua"), Some(fleet_id))?;
        match self.process_children(fleet_id, writer) {
            Ok(mut children) => {
                writer.write_progress(
                    &format!("Found {} child fleets", children.len()),
                    Some("fua"),
                    Some(fleet_id)
                )?;
                result.append(&mut children);
            }
            Err(e) => {
                writer.write_progress(
                    &format!("Failed to process children for fleet {}: {}", fleet_id, e),
                    Some("fua"),
                    Some(fleet_id)
                )?;
            }
        }

        writer.write_progress("FUA data processing complete", Some("fua"), Some(fleet_id))?;
        
        Ok(json!(result))
    }
}

impl<'a> Extractor for FuaExtractor<'a> {
    fn process_user<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value> {
        writer.write_progress(&format!("Processing FUA data for user: {}", user), Some("fua"), Some(user))?;
        
        // Get user's fleets
        writer.write_progress("Fetching user fleets...", Some("fua"), Some(user))?;
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://fua.corp.amazon.com/api/users/{}/fleets", user),
                "headers": {
                    "referer": "https://fua.corp.amazon.com/"
                }
            }),
        )?;

        // Parse fleets response
        let fleets_data: Value = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse fleets response: {}", e)))?;
            
        // Get user's permissions
        writer.write_progress("Fetching user permissions...", Some("fua"), Some(user))?;
        let permissions_response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum,
            json!({
                "url": format!("https://fua.corp.amazon.com/api/users/{}/permissions", user),
                "headers": {
                    "referer": "https://fua.corp.amazon.com/"
                }
            }),
        )?;

        // Parse permissions response
        let permissions_data: Value = serde_json::from_str(&permissions_response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse permissions response: {}", e)))?;

        // Combine all data
        let result = json!({
            "fleets": fleets_data,
            "permissions": permissions_data,
            "user": user
        });

        writer.write_progress("FUA data processing complete for user", Some("fua"), Some(user))?;
        
        Ok(result)
    }
}
