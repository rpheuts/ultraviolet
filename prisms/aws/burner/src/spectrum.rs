//! Typed structures for the burner prism spectrum.
//!
//! This module defines the input and output structures for the burner prism,
//! providing type safety and better developer experience.

use serde::{Deserialize, Serialize};

// Common types

/// Account information returned from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    #[serde(rename = "accountName")]
    pub account_name: String,
    
    #[serde(rename = "awsAccountId")]
    pub aws_account_id: Option<String>,
    
    pub status: String,
    
    #[serde(rename = "validTill")]
    pub valid_till: String,
    
    #[serde(default)]
    pub user: Option<String>,
}

// List frequency

/// Input for the list frequency - empty object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListInput {}

/// Output from the list frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOutput {
    pub accounts: Vec<Account>,
}

// Create frequency

/// Input for the create frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInput {
    pub name: String,
}

/// Output from the create frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOutput {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

// Delete frequency

/// Input for the delete frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteInput {
    pub name: String,
}

/// Output from the delete frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOutput {
    pub success: bool,
    pub message: String,
}

// URL frequency

/// Input for the url frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlInput {
    pub account_id: String,
}

/// Output from the url frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlOutput {
    pub url: String,
}

// Refraction types

/// Response from the curl prism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: i32,
    pub body: String,
}

/// Token response structure from the CSRF endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
}
