//! Oppie prism implementation for the Ultraviolet system.
//!
//! This prism provides capabilities for extracting and uploading data from
//! various internal services.

pub mod spectrum;
pub mod writer;
pub mod extractors;

use std::env;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use serde_json::json;

use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum, PrismMultiplexer
};

use crate::writer::ExtractorWriter;
use crate::extractors::Extractor;

use crate::spectrum::{ExtractInput, UploadInput};
use crate::writer::LinkWriter;
use crate::extractors::{
    SasExtractor, ShepherdExtractor, PolicyEngineExtractor, 
    CtiExtractor, OrgExtractor, FuaExtractor, AsrExtractor
};

/// Oppie prism for extracting and uploading data from internal services.
pub struct OppiePrism {
    spectrum: Option<UVSpectrum>,
    multiplexer: PrismMultiplexer,
}

impl OppiePrism {
    /// Create a new oppie prism.
    pub fn new() -> Self {
        Self {
            spectrum: None,
            multiplexer: PrismMultiplexer::new(),
        }
    }
    
    /// Get the data directory path.
    fn get_data_dir(&self) -> Result<PathBuf> {
        let home_dir = env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| UVError::ExecutionError(format!("Failed to get home directory: {}", e)))?;
            
        let data_dir = home_dir.join(".uv/prisms/aws/oppie/data");
        
        // Create the directory if it doesn't exist
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)
                .map_err(|e| UVError::ExecutionError(format!("Failed to create data directory: {}", e)))?;
        }
        
        Ok(data_dir)
    }
    
    /// Upload a file to the Oppie backend.
    fn upload_file(&self, path: &PathBuf, type_: &str, org: &str, writer: &mut LinkWriter) -> Result<()> {
        writer.write_line(&format!("Uploading {} for {}", type_, org))?;
        
        let content = fs::read_to_string(path)
            .map_err(|e| UVError::ExecutionError(format!("Failed to read file {}: {}", path.display(), e)))?;
        
        let response = self.multiplexer.refract_and_absorb::<spectrum::HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": format!("https://api.ce.kindle.amazon.dev/upload?type={}&org={}", type_, org),
                "headers": {
                    "referer": "https://console.harmony.a2z.com/",
                    "content-type": "application/json"
                },
                "body": content
            }),
        )?;

        writer.write_line(&format!("Upload complete: {} (status: {})", type_, response.status))?;
        
        Ok(())
    }
    
    /// Handle the extract frequency.
    fn handle_extract(&self, id: Uuid, input: ExtractInput, link: &UVLink) -> Result<()> {
        // Parse users and services
        let users: Vec<String> = input.users
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
            
        let services: Vec<String> = input.services
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
            
        // Create writer
        let mut writer = LinkWriter::new(link, id);
        
        // Get data directory
        let data_dir = self.get_data_dir()?;
        
        // Create extractors
        let spectrum = self.spectrum.as_ref().unwrap();
        
        // Process each user
        for user in &users {
            // Process SAS data if requested
            if services.contains(&"sas".to_string()) {
                let sas_extractor = SasExtractor::new(&self.multiplexer, spectrum);
                let sas_result = sas_extractor.process_user(user, &mut writer)?;
                
                // Save SAS data
                let sas_file = data_dir.join(format!("{}_sas.json", user));
                fs::write(&sas_file, serde_json::to_string_pretty(&sas_result)?)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to write SAS data: {}", e)))?;
            }
            
            // Process Shepherd data if requested
            if services.contains(&"shepherd".to_string()) {
                let shepherd_extractor = ShepherdExtractor::new(&self.multiplexer, spectrum);
                let shepherd_result = shepherd_extractor.process_user(user, &mut writer)?;
                
                // Save Shepherd data
                let shepherd_file = data_dir.join(format!("{}_shepherd.json", user));
                fs::write(&shepherd_file, serde_json::to_string_pretty(&shepherd_result)?)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to write Shepherd data: {}", e)))?;
            }
            
            // Process PolicyEngine data if requested
            if services.contains(&"policy_engine".to_string()) {
                let policy_engine_extractor = PolicyEngineExtractor::new(&self.multiplexer, spectrum);
                let policy_engine_result = policy_engine_extractor.process_user(user, &mut writer)?;
                
                // Save PolicyEngine data
                let policy_engine_file = data_dir.join(format!("{}_policy_engine.csv", user));
                if let Some(csv) = policy_engine_result.get("csv").and_then(|v| v.as_str()) {
                    fs::write(&policy_engine_file, csv)
                        .map_err(|e| UVError::ExecutionError(format!("Failed to write PolicyEngine data: {}", e)))?;
                }
            }
            
            // Process CTI data if requested
            if services.contains(&"cti".to_string()) {
                let cti_extractor = CtiExtractor::new(&self.multiplexer, spectrum);
                let cti_result = cti_extractor.process_user(user, &mut writer)?;
                
                // Save CTI data
                let cti_file = data_dir.join(format!("{}_cti.json", user));
                fs::write(&cti_file, serde_json::to_string_pretty(&cti_result)?)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to write CTI data: {}", e)))?;
            }
            
            // Process ASR data if requested
            if services.contains(&"asr".to_string()) {
                let asr_extractor = AsrExtractor::new(&self.multiplexer, spectrum);
                let asr_result = asr_extractor.process_user(user, &mut writer)?;
                
                // Save ASR data
                let asr_file = data_dir.join(format!("{}_asr.json", user));
                fs::write(&asr_file, serde_json::to_string_pretty(&asr_result)?)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to write ASR data: {}", e)))?;
            }
        }
        
        // Process Org data if requested (one file for all users)
        if services.contains(&"org".to_string()) {
            let org_extractor = OrgExtractor::new(&self.multiplexer, spectrum);
            let org_result = org_extractor.process_users(users.clone(), &mut writer)?;
            
            // Save Org data
            let org_file = data_dir.join("org.json"); // One file for all users
            fs::write(&org_file, serde_json::to_string_pretty(&org_result)?)
                .map_err(|e| UVError::ExecutionError(format!("Failed to write Org data: {}", e)))?;
        }
        
        // Process FUA data if requested and fleet_id provided
        if services.contains(&"fua".to_string()) {
            if let Some(fleet_id) = &input.fleet_id {
                let fua_extractor = FuaExtractor::new(&self.multiplexer, spectrum);
                let fua_result = fua_extractor.process_fleet_id(fleet_id, &mut writer)?;
                
                // Save FUA data
                let fua_file = data_dir.join(format!("{}_fua.json", fleet_id));
                fs::write(&fua_file, serde_json::to_string_pretty(&fua_result)?)
                    .map_err(|e| UVError::ExecutionError(format!("Failed to write FUA data: {}", e)))?;
            }
        }
        
        // Signal successful completion
        writer.write_line(&format!("Data extraction complete. Files saved to {}", data_dir.display()))?;
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle the upload frequency.
    fn handle_upload(&self, id: Uuid, input: UploadInput, link: &UVLink) -> Result<()> {
        // Parse users and services
        let users: Vec<String> = input.users
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
            
        let services: Vec<String> = input.services
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
            
        // Create writer
        let mut writer = LinkWriter::new(link, id);
        
        // Get data directory
        let data_dir = self.get_data_dir()?;
        
        let mut uploaded = Vec::new();
        
        // Process each user
        for user in &users {
            // Upload SAS data if requested
            if services.contains(&"sas".to_string()) {
                let file = data_dir.join(format!("{}_sas.json", user));
                if file.exists() {
                    self.upload_file(&file, "oppie.sas", user, &mut writer)?;
                    uploaded.push(format!("sas for {}", user));
                }
            }
            
            // Upload Shepherd data if requested
            if services.contains(&"shepherd".to_string()) {
                let file = data_dir.join(format!("{}_shepherd.json", user));
                if file.exists() {
                    self.upload_file(&file, "oppie.shepherd", user, &mut writer)?;
                    uploaded.push(format!("shepherd for {}", user));
                }
            }
            
            // Upload PolicyEngine data if requested
            if services.contains(&"policy_engine".to_string()) {
                let file = data_dir.join(format!("{}_policy_engine.csv", user));
                if file.exists() {
                    self.upload_file(&file, "oppie.pe", user, &mut writer)?;
                    uploaded.push(format!("policy_engine for {}", user));
                }
            }
            
            // Upload CTI data if requested
            if services.contains(&"cti".to_string()) {
                let file = data_dir.join(format!("{}_cti.json", user));
                if file.exists() {
                    self.upload_file(&file, "oppie.cti", user, &mut writer)?;
                    uploaded.push(format!("cti for {}", user));
                }
            }
            
            // Upload ASR data if requested
            if services.contains(&"asr".to_string()) {
                let file = data_dir.join(format!("{}_asr.json", user));
                if file.exists() {
                    self.upload_file(&file, "oppie.asr", user, &mut writer)?;
                    uploaded.push(format!("asr for {}", user));
                }
            }
        }
        
        // Upload Org data if requested (one file for all users)
        if services.contains(&"org".to_string()) {
            let file = data_dir.join("org.json");
            if file.exists() {
                // Use first user as org for org data
                if let Some(first_user) = users.first() {
                    self.upload_file(&file, "oppie.org", first_user, &mut writer)?;
                    uploaded.push("org".to_string());
                }
            }
        }
        
        // Upload FUA data if requested and fleet_id provided
        if services.contains(&"fua".to_string()) {
            if let Some(fleet_id) = &input.fleet_id {
                let file = data_dir.join(format!("{}_fua.json", fleet_id));
                if file.exists() {
                    // Use first user as org for FUA data
                    if let Some(first_user) = users.first() {
                        self.upload_file(&file, "oppie.fua", first_user, &mut writer)?;
                        uploaded.push(format!("fua for fleet {}", fleet_id));
                    }
                }
            }
        }
        
        // Signal successful completion
        writer.write_line(&format!("Upload complete: {}", uploaded.join(", ")))?;
        link.emit_trap(id, None)?;
        
        Ok(())
    }
}

impl UVPrism for OppiePrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "extract" => {
                    // Deserialize the input
                    let input: ExtractInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the extract request
                    self.handle_extract(id, input, link)?;
                    return Ok(true);
                },
                "upload" => {
                    // Deserialize the input
                    let input: UploadInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the upload request
                    self.handle_upload(id, input, link)?;
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
    Box::new(OppiePrism::new())
}
