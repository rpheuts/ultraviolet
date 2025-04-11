//! CDK prism implementation for the Ultraviolet system.
//!
//! This prism provides utilities for working with AWS CDK projects,
//! such as listing resources from CloudFormation templates.

pub mod spectrum;
pub mod aws;
pub mod resources;

use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use glob::glob;
use uuid::Uuid;
use tokio::runtime::Runtime;

use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

use crate::spectrum::{ResourcesInput, CleanupInput, CleanupResult, CdkResource, CloudFormationTemplate, CdkManifest};
use crate::resources::{ResourceHandler, get_handler, StatusFilter, COMMON_RESOURCE_TYPES};

/// Resource information containing status and physical ID
#[derive(Debug, Clone)]
struct ResourceInfo {
    status: String,
    physical_id: Option<String>,
}

/// CDK prism for AWS CDK utilities.
pub struct CDKPrism {
    spectrum: Option<UVSpectrum>,
    runtime: Runtime,
    client: aws_sdk_cloudformation::Client,
}

impl CDKPrism {
    /// Create a new CDK prism.
    pub fn new() -> Result<Self> {
        // Create a tokio runtime
        let runtime = Runtime::new()
            .map_err(|e| UVError::Other(format!("Failed to create runtime: {}", e)))?;
        
        // Initialize AWS client with default region
        let client = runtime.block_on(async {
            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region("us-east-1")
                .load()
                .await;

            aws_sdk_cloudformation::Client::new(&config)
        });

        Ok(Self {
            spectrum: None,
            runtime,
            client,
        })
    }
    
    /// Extract stack name mappings from the CDK manifest.json file
    fn extract_stack_names(&self, cdk_out_path: &Path) -> Result<HashMap<String, String>> {
        let manifest_path = cdk_out_path.join("manifest.json");
        if !manifest_path.exists() {
            return Ok(HashMap::new());
        }
        
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| UVError::Other(format!("Failed to read manifest file: {}", e)))?;
            
        let manifest: CdkManifest = serde_json::from_str(&content)
            .map_err(|e| UVError::Other(format!("Failed to parse manifest file: {}", e)))?;
        
        let mut stack_mappings = HashMap::new();
        
        for (_, artifact) in manifest.artifacts {
            if artifact.artifact_type == "aws:cloudformation:stack" {
                if let Some(properties) = artifact.properties {
                    if let (Some(template_file), Some(stack_name)) = (properties.template_file, properties.stack_name) {
                        // Map from template filename to actual stack name
                        stack_mappings.insert(template_file, stack_name);
                    }
                }
            }
        }
        
        Ok(stack_mappings)
    }
    
    /// Get resource statuses for a CloudFormation stack
    async fn get_stack_resources(&self, stack_name: &str, _region: &str) -> Result<HashMap<String, ResourceInfo>> {
        // Call the CloudFormation API
        let response = self.client
            .describe_stack_resources()
            .stack_name(stack_name)
            .send()
            .await
            .map_err(|e| UVError::Other(format!("CloudFormation API error: {}", e)))?;

        // Map logical IDs to statuses and physical IDs
        let mut resource_infos = HashMap::new();
        let resources = response.stack_resources();
        for resource in resources {
            if let (Some(logical_id), Some(status)) = (resource.logical_resource_id(), resource.resource_status()) {
                let physical_id = resource.physical_resource_id().map(|id| id.to_string());
                
                resource_infos.insert(
                    logical_id.to_string(),
                    ResourceInfo {
                        status: status.as_str().to_string(),
                        physical_id,
                    },
                );
            }
        }
        
        Ok(resource_infos)
    }

    /// Parse CloudFormation templates from a cdk.out directory
    /// and extract resources.
    fn handle_resources(&self, id: Uuid, input: ResourcesInput, link: &UVLink) -> Result<()> {
        let cdk_out_path = Path::new(&input.cdk_out_path);
        
        // Ensure the cdk.out directory exists
        if !cdk_out_path.exists() || !cdk_out_path.is_dir() {
            return Err(UVError::InvalidInput(format!(
                "CDK output directory not found at path: {}", input.cdk_out_path
            )));
        }
        
        // Extract stack names from manifest.json
        let stack_mappings = self.extract_stack_names(cdk_out_path)?;
        
        // Find all CloudFormation template files
        let pattern = format!("{}/*.template.json", cdk_out_path.display());
        let template_paths = glob(&pattern)
            .map_err(|e| UVError::Other(format!("Failed to search for templates: {}", e)))?;
        
        // Check if we need to fetch resource statuses
        let check_status = input.check_status.unwrap_or(false);
        let region = input.region.as_deref().unwrap_or("us-east-1").to_string();
        
        // Process each template file
        let mut found_templates = false;
        for path_result in template_paths {
            let path = path_result
                .map_err(|e| UVError::Other(format!("Error reading template path: {}", e)))?;
            
            // Get the template filename
            let template_filename = path.file_name()
                .and_then(|f| f.to_str())
                .unwrap_or_default();
            
            // Extract stack name - prefer the one from manifest if available
            let template_basename = path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| stem.replace(".template", ""))
                .unwrap_or_else(|| "unknown".to_string());
                
            // Use actual CloudFormation stack name from manifest if available
            let stack_name = stack_mappings.get(template_filename)
                .cloned()
                .unwrap_or(template_basename);
            
            // If stack filter is provided, skip non-matching stacks
            if let Some(ref stack_filter) = input.stack {
                if !stack_name.contains(stack_filter) {
                    continue;
                }
            }
            
            found_templates = true;
            
            // If status checking is enabled, get resource statuses for this stack
            let resource_statuses = if check_status {
                match self.runtime.block_on(async {
                    self.get_stack_resources(&stack_name, &region).await
                }) {
                    Ok(statuses) => Some(statuses),
                    Err(e) => {
                        // Log error but continue without statuses
                        log::warn!("Failed to get resource statuses for stack {}: {}", stack_name, e);
                        None
                    }
                }
            } else {
                None
            };
            
            self.process_template_file(&path, &stack_name, id, link, resource_statuses.as_ref())?;
        }
        
        if !found_templates {
            return Err(UVError::InvalidInput(format!(
                "No CloudFormation templates found in directory: {}", input.cdk_out_path
            )));
        }
        
        // Signal successful completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Process a single CloudFormation template file
    fn process_template_file(
        &self, 
        path: &PathBuf, 
        stack_name: &str, 
        id: Uuid, 
        link: &UVLink, 
        resource_statuses: Option<&HashMap<String, ResourceInfo>>
    ) -> Result<()> {
        // Read and parse the template file
        let content = fs::read_to_string(path)
            .map_err(|e| UVError::Other(format!("Failed to read template file {}: {}", path.display(), e)))?;
            
        let template: CloudFormationTemplate = serde_json::from_str(&content)
            .map_err(|e| UVError::Other(format!("Failed to parse template file {}: {}", path.display(), e)))?;
        
        // Extract resources from the template
        if let Some(resources) = template.resources {
            for (logical_id, resource) in resources {            
                // Get status and physical ID if available
                let (status, physical_id) = if let Some(statuses) = resource_statuses {
                    if let Some(info) = statuses.get(&logical_id) {
                        (Some(info.status.clone()), info.physical_id.clone())
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };
                
                // Create and emit resource
                let cdk_resource = CdkResource {
                    logical_id,
                    resource_type: resource.resource_type,
                    stack: stack_name.to_string(),
                    status,
                    physical_id,
                };
                
                // Serialize and emit the resource
                link.emit_photon(id, serde_json::to_value(cdk_resource)?)?;
            }
        }
        
        Ok(())
    }

    /// Handle cleanup of resources based on filter criteria
    fn handle_cleanup(&self, id: Uuid, input: CleanupInput, link: &UVLink) -> Result<()> {
        let cdk_out_path = Path::new(&input.cdk_out_path);
        
        // Ensure the cdk.out directory exists
        if !cdk_out_path.exists() || !cdk_out_path.is_dir() {
            return Err(UVError::InvalidInput(format!(
                "CDK output directory not found at path: {}", input.cdk_out_path
            )));
        }
        
        // Extract stack names from manifest.json
        let stack_mappings = self.extract_stack_names(cdk_out_path)?;
        
        // Find all CloudFormation template files
        let pattern = format!("{}/*.template.json", cdk_out_path.display());
        let template_paths = glob(&pattern)
            .map_err(|e| UVError::Other(format!("Failed to search for templates: {}", e)))?;
        
        // Prepare filter criteria
        let dry_run = input.dry_run.unwrap_or(true);
        let region = input.region.clone();
        
        // Parse status filter
        let status_filter = match input.status_filter.as_deref() {
            Some("DELETE_FAILED") => StatusFilter::DeleteFailed,
            Some("ALL") => StatusFilter::All,
            _ => StatusFilter::DeleteSkipped,
        };
        
        // Get optional resource type filter
        let resource_types = input.resource_types.unwrap_or_else(|| {
            // Default to common resource types
            COMMON_RESOURCE_TYPES.iter().map(|s| s.to_string()).collect()
        });
        
        // Collect resources to cleanup
        let mut resources_to_cleanup = Vec::new();
        
        // Process each template file
        let mut found_templates = false;
        for path_result in template_paths {
            let path = path_result
                .map_err(|e| UVError::Other(format!("Error reading template path: {}", e)))?;
            
            // Get the template filename
            let template_filename = path.file_name()
                .and_then(|f| f.to_str())
                .unwrap_or_default();
            
            // Extract stack name from filename and mapping
            let template_basename = path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| stem.replace(".template", ""))
                .unwrap_or_else(|| "unknown".to_string());
                
            let stack_name = stack_mappings.get(template_filename)
                .cloned()
                .unwrap_or(template_basename);
            
            // If stack filter is provided, skip non-matching stacks
            if let Some(ref stack_filter) = input.stack {
                if !stack_name.contains(stack_filter) {
                    continue;
                }
            }
            
            found_templates = true;
            
            // Get resource statuses for this stack
            let resource_statuses = match self.runtime.block_on(async {
                self.get_stack_resources(&stack_name, region.as_deref().unwrap_or("us-east-1")).await
            }) {
                Ok(statuses) => Some(statuses),
                Err(e) => {
                    log::warn!("Failed to get resource statuses for stack {}: {}", stack_name, e);
                    None
                }
            };
            
            // Filter and collect resources for cleanup
            self.collect_resources_for_cleanup(
                &path, 
                &stack_name, 
                resource_statuses.as_ref(),
                &resource_types,
                &status_filter,
                &mut resources_to_cleanup
            )?;
        }
        
        if !found_templates {
            return Err(UVError::InvalidInput(format!(
                "No CloudFormation templates found in directory: {}", input.cdk_out_path
            )));
        }
        
        // Process each resource for cleanup
        for (logical_id, stack_name, resource_type, physical_id, status) in resources_to_cleanup {
            let result = if let Some(handler) = get_handler(&resource_type) {
                if let Some(physical_id) = &physical_id {
                    // Execute cleanup
                    match self.runtime.block_on(async {
                        handler.delete_resource(physical_id, region.as_deref(), dry_run).await
                    }) {
                        Ok(cleanup_status) => {
                            cleanup_status.to_string()
                        },
                        Err(e) => {
                            format!("ERROR: {}", e)
                        }
                    }
                } else {
                    "SKIPPED: No physical ID available".to_string()
                }
            } else {
                format!("SKIPPED: No handler for resource type {}", resource_type)
            };
            
            // Create and emit cleanup result
            let cleanup_result = CleanupResult {
                logical_id,
                resource_type,
                stack: stack_name,
                status,
                physical_id,
                cleanup_result: result,
            };
            
            // Emit the result
            link.emit_photon(id, serde_json::to_value(cleanup_result)?)?;
        }
        
        // Signal successful completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Collect resources that match the cleanup filters
    fn collect_resources_for_cleanup(
        &self,
        path: &PathBuf,
        stack_name: &str,
        resource_statuses: Option<&HashMap<String, ResourceInfo>>,
        resource_types: &[String],
        status_filter: &StatusFilter,
        resources_to_cleanup: &mut Vec<(String, String, String, Option<String>, Option<String>)>
    ) -> Result<()> {
        // Read and parse the template file
        let content = fs::read_to_string(path)
            .map_err(|e| UVError::Other(format!("Failed to read template file {}: {}", path.display(), e)))?;
            
        let template: CloudFormationTemplate = serde_json::from_str(&content)
            .map_err(|e| UVError::Other(format!("Failed to parse template file {}: {}", path.display(), e)))?;
        
        // Extract resources from the template
        if let Some(resources) = template.resources {
            for (logical_id, resource) in resources {
                // Skip if resource type doesn't match filter
                if !resource_types.iter().any(|t| *t == resource.resource_type) {
                    continue;
                }
                
                // Get status and physical ID if available
                let (status, physical_id) = if let Some(statuses) = resource_statuses {
                    if let Some(info) = statuses.get(&logical_id) {
                        (Some(info.status.clone()), info.physical_id.clone())
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };
                
                // Check if the resource matches the status filter
                if !status_filter.matches(status.as_deref()) {
                    continue;
                }
                
                // Add to cleanup list
                resources_to_cleanup.push((
                    logical_id,
                    stack_name.to_string(),
                    resource.resource_type,
                    physical_id,
                    status,
                ));
            }
        }
        
        Ok(())
    }
}

impl UVPrism for CDKPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }

    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "resources" => {
                    // Deserialize the input
                    let input: ResourcesInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the resources request
                    self.handle_resources(id, input, link)?;
                    return Ok(true);
                },
                "cleanup" => {
                    // Deserialize the input
                    let input: CleanupInput = serde_json::from_value(wavefront.input.clone())?;
                    
                    // Handle the cleanup request
                    self.handle_cleanup(id, input, link)?;
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
    match CDKPrism::new() {
        Ok(prism) => Box::new(prism),
        Err(e) => {
            log::error!("Failed to create CDK prism: {}", e);
            // Fallback to a minimal implementation that reports the error
            Box::new(ErrorPrism::new(format!("Failed to create CDK prism: {}", e)))
        }
    }
}

/// Minimal prism implementation that simply reports an error
struct ErrorPrism {
    error_message: String,
    spectrum: Option<UVSpectrum>,
}

impl ErrorPrism {
    fn new(error_message: String) -> Self {
        Self {
            error_message,
            spectrum: None,
        }
    }
}

impl UVPrism for ErrorPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }

    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(_wavefront) = pulse {
            // Return the initialization error for any request
            let error = UVError::ExecutionError(self.error_message.clone());
            link.emit_trap(id, Some(error))?;
            return Ok(true);
        }
        
        Ok(false)
    }
}
