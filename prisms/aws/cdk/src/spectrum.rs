use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input parameters for the resources frequency
#[derive(Debug, Deserialize)]
pub struct ResourcesInput {
    pub cdk_out_path: String,
    pub stack: Option<String>,
    pub check_status: Option<bool>,
    pub region: Option<String>,
}

/// Input parameters for the cleanup frequency
#[derive(Debug, Deserialize)]
pub struct CleanupInput {
    pub cdk_out_path: String,
    pub stack: Option<String>,
    pub resource_types: Option<Vec<String>>,
    pub status_filter: Option<String>,
    pub region: Option<String>,
    pub dry_run: Option<bool>,
}

/// Result of a resource cleanup operation
#[derive(Debug, Serialize)]
pub struct CleanupResult {
    pub logical_id: String,
    pub stack: String,
    pub physical_id: Option<String>,
    pub resource_type: String,
    pub status: Option<String>,
    pub cleanup_result: String,
}

/// A resource extracted from a CloudFormation template
#[derive(Debug, Serialize)]
pub struct CdkResource {
    pub logical_id: String,
    pub resource_type: String,
    pub stack: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_id: Option<String>,
}

/// Structure representing a CloudFormation template
#[derive(Debug, Deserialize)]
pub struct CloudFormationTemplate {
    #[serde(rename = "Resources")]
    pub resources: Option<HashMap<String, CloudFormationResource>>,
}

/// Structure representing a resource in a CloudFormation template
#[derive(Debug, Deserialize)]
pub struct CloudFormationResource {
    #[serde(rename = "Type")]
    pub resource_type: String,
    
    #[serde(rename = "Properties")]
    pub properties: Option<serde_json::Value>,
}

/// Structure for parsing the CDK manifest.json file
#[derive(Debug, Deserialize)]
pub struct CdkManifest {
    pub artifacts: HashMap<String, CdkArtifact>,
}

/// Structure for a CDK artifact in the manifest
#[derive(Debug, Deserialize)]
pub struct CdkArtifact {
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub properties: Option<CdkArtifactProperties>,
}

/// Properties of a CloudFormation stack artifact
#[derive(Debug, Deserialize)]
pub struct CdkArtifactProperties {
    #[serde(rename = "templateFile")]
    pub template_file: Option<String>,
    #[serde(rename = "stackName")]
    pub stack_name: Option<String>,
}
