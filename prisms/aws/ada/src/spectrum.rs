//! Typed structures for the ada prism spectrum.
//!
//! This module defines the input and output structures for the ada prism,
//! providing type safety and better developer experience.

use serde::{Deserialize, Serialize};

// Common types and refraction types

/// Response from the curl prism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: i32,
    pub body: String,
}

/// Token response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
}

// Credentials frequency

/// Input for the credentials frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialsInput {
    pub account: String,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_provider() -> String {
    "conduit".to_string()
}

fn default_role() -> String {
    "IibsAdminAccess-DO-NOT-DELETE".to_string()
}

/// Output from the credentials frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialsOutput {
    pub success: bool,
    pub message: String,
}

// Status frequency

/// Input for the status frequency - empty object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusInput {}

/// Status process information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub account: String,
    pub started_at: String,
    pub pid: i32,
}

// Provision frequency

/// Input for the provision frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionInput {
    pub account: String,
}

/// Output from the provision frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionOutput {
    pub success: bool,
    pub message: String,
}

// Admin frequency

/// Input for the admin frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminInput {
    #[serde(default = "default_account")]
    pub account: String,
}

fn default_account() -> String {
    "187792406069".to_string()
}

/// Output from the admin frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminOutput {
    pub success: bool,
    pub message: String,
}
