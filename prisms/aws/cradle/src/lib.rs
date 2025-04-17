//! Cradle prism implementation for the Ultraviolet system.
//!
//! This prism provides capabilities for interacting with the Cradle service.

pub mod models;
pub mod spectrum;

use serde_json::json;
use chrono::{TimeZone, Utc};

use uuid::Uuid;
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum, PrismMultiplexer
};

use crate::models::{Profile, Job, ProfileOutput, JobOutput};
use crate::spectrum::{
    HttpResponse, ProfilesListInput, ProfilesGetInput, JobsListInput, JobsGetInput
};

/// Cradle prism for interacting with the Cradle service.
pub struct CradlePrism {
    spectrum: Option<UVSpectrum>,
    multiplexer: PrismMultiplexer,
}

impl CradlePrism {
    /// Create a new cradle prism.
    pub fn new() -> Self {
        Self {
            spectrum: None,
            multiplexer: PrismMultiplexer::new(),
        }
    }
    
    /// Format a timestamp as a human-readable string.
    fn format_timestamp(&self, ts: &models::Timestamp) -> String {
        if let Some(dt) = Utc.timestamp_opt(ts.seconds, ts.nanos as u32).single() {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            "Invalid date".to_string()
        }
    }
    
    /// Handle the profiles_list frequency.
    fn handle_profiles_list(&self, id: Uuid, input: ProfilesListInput, link: &UVLink) -> Result<()> {
        let account_name = input.account_name;
        
        // Make request to Cradle API
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://api.ce.kindle.amazon.dev/governance?org=hottier&action=DryadOperation&marketplace=1",
                "headers": {
                    "referer": "https://console.harmony.a2z.com/",
                    "content-type": "application/json"
                },
                "body": json!({
                    "accountName": account_name,
                    "operation": "getProfiles"
                }).to_string()
            }),
        )?;

        // Parse array of JSON strings
        let profiles_json: Vec<String> = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse profiles array: {}", e)))?;

        // Parse each profile string into Profile struct
        for profile_str in profiles_json {
            let profile: Profile = serde_json::from_str(&profile_str)
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse profile: {}", e)))?;

                link.emit_photon(id, json!(ProfileOutput {
                id: profile.id,
                name: profile.name,
                description: profile.description,
                profile_state: profile.profile_state,
                created_by: profile.created_by,
                last_updated_date: self.format_timestamp(&profile.last_updated_date),
            }))?;
        }

        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle the profiles_get frequency.
    fn handle_profiles_get(&self, id: Uuid, input: ProfilesGetInput, link: &UVLink) -> Result<()> {
        let profile_id = input.profile_id;
        
        // Make request to Cradle API
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://api.ce.kindle.amazon.dev/governance?org=hottier&action=DryadOperation&marketplace=1",
                "headers": {
                    "referer": "https://console.harmony.a2z.com/",
                    "content-type": "application/json"
                },
                "body": json!({
                    "profileId": profile_id,
                    "operation": "getProfile"
                }).to_string()
            }),
        )?;

        // Parse profile JSON
        let profile: Profile = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse profile: {}", e)))?;

        // Transform data for display
        let result = json!({
            "name": profile.name,
            "description": profile.description,
            "accountName": profile.account_name,
            "profileState": profile.profile_state,
            "version": profile.version,
            "createdBy": profile.created_by,
            "lastUpdatedDate": self.format_timestamp(&profile.last_updated_date),
            "closure": profile.closure
        });

        // Emit result
        link.emit_photon(id, result)?;
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle the jobs_list frequency.
    fn handle_jobs_list(&self, id: Uuid, input: JobsListInput, link: &UVLink) -> Result<()> {
        let profile_id = input.profile_id;
        
        // Make request to Cradle API
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://api.ce.kindle.amazon.dev/governance?org=hottier&action=DryadOperation&marketplace=1",
                "headers": {
                    "referer": "https://console.harmony.a2z.com/",
                    "content-type": "application/json"
                },
                "body": json!({
                    "profileId": profile_id,
                    "operation": "getJobs"
                }).to_string()
            }),
        )?;

        // Parse array of JSON strings
        let jobs_json: Vec<String> = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse jobs array: {}", e)))?;

        // Parse each job string into Job struct
        for job_str in jobs_json {
            let job: Job = serde_json::from_str(&job_str)
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse job: {}", e)))?;

                link.emit_photon(id, json!(JobOutput {
                id: job.id,
                name: job.name,
                created_by: job.created_by,
                last_updated_date: self.format_timestamp(&job.last_updated_date),
                status: if job.job_parameters.disabled { "Disabled".into() } else { "Enabled".into() },
            }))?;
        }

        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle the jobs_get frequency.
    fn handle_jobs_get(&self, id: Uuid, input: JobsGetInput, link: &UVLink) -> Result<()> {
        let profile_id = input.profile_id;
        let job_id = input.job_id;
        
        // Make request to Cradle API
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://api.ce.kindle.amazon.dev/governance?org=hottier&action=DryadOperation&marketplace=1",
                "headers": {
                    "referer": "https://console.harmony.a2z.com/",
                    "content-type": "application/json"
                },
                "body": json!({
                    "profileId": profile_id,
                    "jobId": job_id,
                    "operation": "getJob"
                }).to_string()
            }),
        )?;

        // Parse job JSON
        let job: Job = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse job: {}", e)))?;

        // Transform timestamps for display
        let mut job_json = serde_json::to_value(&job)?;
        if let Some(obj) = job_json.as_object_mut() {
            obj.insert("lastUpdatedDate".into(), json!(self.format_timestamp(&job.last_updated_date)));
        }

        // Emit result
        link.emit_photon(id, job_json)?;
        link.emit_trap(id, None)?;
        
        Ok(())
    }
}

impl UVPrism for CradlePrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "profilesList" => {
                    // Deserialize the input
                    let input: ProfilesListInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the request
                    self.handle_profiles_list(id, input, link)?;
                    return Ok(true);
                },
                "profilesGet" => {
                    // Deserialize the input
                    let input: ProfilesGetInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the request
                    self.handle_profiles_get(id, input, link)?;
                    return Ok(true);
                },
                "jobsList" => {
                    // Deserialize the input
                    let input: JobsListInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the request
                    self.handle_jobs_list(id, input, link)?;
                    return Ok(true);
                },
                "jobsGet" => {
                    // Deserialize the input
                    let input: JobsGetInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the request
                    self.handle_jobs_get(id, input, link)?;
                    return Ok(true);
                },
                _ => {
                    // Unknown frequency
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
}

// Export a function to create a new instance
// This will be used by the dynamic loading system
#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(CradlePrism::new())
}
