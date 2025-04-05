//! Burner prism implementation for the Ultraviolet system.
//!
//! This prism provides capabilities for managing burner AWS accounts through
//! the internal Amazon Conduit API.

pub mod spectrum;

use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

use uv_core::{
    Result, UVError, UVLink, UVPrism, PrismMultiplexer, UVPulse, UVSpectrum,
};

use crate::spectrum::{
    Account, CreateInput, CreateOutput, DeleteInput, DeleteOutput,
    HttpResponse, ListInput, TokenResponse, UrlInput, UrlOutput,
};

/// Burner prism for managing AWS accounts
pub struct BurnerPrism {
    spectrum: Option<UVSpectrum>,
    multiplexer: PrismMultiplexer,
}

impl BurnerPrism {
    /// Create a new burner prism.
    pub fn new() -> Self {
        Self {
            spectrum: None,
            multiplexer: PrismMultiplexer::new(),
        }
    }

    /// Get CSRF token required for mutation operations
    fn get_csrf_token(&self) -> Result<String> {
        // Use the curl.get refraction
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://conduit.security.a2z.com/api/token"
            })
        )?;
        
        // Parse the token from the response
        let token_response: TokenResponse = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse token response: {}", e)))?;
            
        Ok(token_response.token)
    }
    
    /// Handle list frequency
    fn handle_list(&self, id: Uuid, _input: ListInput, link: &UVLink) -> Result<()> {
        // Use the curl.get refraction to get burner accounts
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://conduit.security.a2z.com/api/accounts/burner"
            })
        )?;
        
        // Parse the accounts from the response
        let accounts_data: Value = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse accounts response: {}", e)))?;
            
        // Create and emit each account as a separate photon
        if let Some(accounts) = accounts_data.get("accounts").and_then(|a| a.as_array()) {
            let account_list: Vec<Account> = serde_json::from_value(accounts.clone().into())
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse account list: {}", e)))?;
                
            // Emit each account as a separate photon
            for account in account_list {
                link.emit_photon(id, serde_json::to_value(account)?)?;
            }
        }
        
        // Signal successful completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle create frequency
    fn handle_create(&self, id: Uuid, input: CreateInput, link: &UVLink) -> Result<()> {
        // Get CSRF token
        let token = self.get_csrf_token()?;
        
        // Prepare headers
        let mut headers = HashMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers.insert("anti-csrftoken-a2z".to_string(), token);
        
        // Use the curl.post refraction to create a burner account
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": format!("https://conduit.security.a2z.com/api/accounts/burner/accountName/{}", input.name),
                "headers": headers,
                "body": "{}"
            })
        )?;
        
        // Parse and handle the response
        let body = &response.body;
        
        // Handle specific error cases
        if body.contains("ActiveAccountsLimitExceededException") {
            let output = CreateOutput {
                success: false,
                message: "You have reached the maximum limit of 2 active burner accounts. Please delete an existing account before creating a new one.".to_string(),
                details: None,
            };
            link.emit_photon(id, serde_json::to_value(output)?)?;
            link.emit_trap(id, None)?;
            return Ok(());
        }
        
        // Parse the response data
        let response_data: Value = serde_json::from_str(body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse response: {}", e)))?;
            
        // Format success response
        if let Some(account) = response_data.get("account") {
            let status = account.get("status").and_then(|s| s.as_str()).unwrap_or("UNKNOWN");
            let valid_till = account.get("validTill").and_then(|s| s.as_str()).unwrap_or("unknown");
            
            let output = CreateOutput {
                success: true,
                message: format!("Account '{}' created with status {} (valid until {})", input.name, status, valid_till),
                details: Some(response_data),
            };
            
            link.emit_photon(id, serde_json::to_value(output)?)?;
        } else {
            let output = CreateOutput {
                success: false,
                message: "Failed to create account: Invalid response format".to_string(),
                details: Some(response_data),
            };
            
            link.emit_photon(id, serde_json::to_value(output)?)?;
        }
        
        // Signal successful completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle delete frequency
    fn handle_delete(&self, id: Uuid, input: DeleteInput, link: &UVLink) -> Result<()> {
        // Get CSRF token
        let token = self.get_csrf_token()?;
        
        // Prepare headers
        let mut headers = HashMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers.insert("anti-csrftoken-a2z".to_string(), token);
        
        // Use the curl.post refraction to delete a burner account
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": format!("https://conduit.security.a2z.com/api/accounts/burner/accountName/{}", input.name),
                "method": "DELETE",
                "headers": headers,
                "body": "{}"
            })
        )?;
        
        let output = if response.status == 200 {
            DeleteOutput {
                success: true,
                message: format!("Successfully deleted burner account: {}", input.name),
            }
        } else {
            // Extract error message from response
            let error_message = if response.body.contains("Exception") {
                response.body.split("message='").nth(1)
                    .and_then(|s| s.split("'").next())
                    .unwrap_or(&response.body)
            } else {
                &response.body
            };
            
            DeleteOutput {
                success: false,
                message: format!("Error deleting account: {}", error_message.trim()),
            }
        };
        
        // Emit the response and completion signal
        link.emit_photon(id, serde_json::to_value(output)?)?;
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle url frequency
    fn handle_url(&self, id: Uuid, input: UrlInput, link: &UVLink) -> Result<()> {
        // Use the curl.get refraction to get console URL
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": format!(
                    "https://conduit.security.a2z.com/api/burner/consoleUrl?awsAccountId={}&awsPartition=aws&sessionDuration=10800", 
                    input.account_id
                )
            })
        )?;
        
        // Parse the URL from the response
        let response_data: Value = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse response: {}", e)))?;
            
        if let Some(url) = response_data.get("url").and_then(|u| u.as_str()) {
            let output = UrlOutput { url: url.to_string() };
            link.emit_photon(id, serde_json::to_value(output)?)?;
            link.emit_trap(id, None)?;
            
            Ok(())
        } else {
            Err(UVError::ExecutionError("No URL found in response".into()))
        }
    }
}

impl UVPrism for BurnerPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "list" => {
                    let input: ListInput = serde_json::from_value(wavefront.input.clone())?;
                    self.handle_list(id, input, link)?;
                    return Ok(true);
                },
                "create" => {
                    let input: CreateInput = serde_json::from_value(wavefront.input.clone())?;
                    self.handle_create(id, input, link)?;
                    return Ok(true);
                },
                "delete" => {
                    let input: DeleteInput = serde_json::from_value(wavefront.input.clone())?;
                    self.handle_delete(id, input, link)?;
                    return Ok(true);
                },
                "url" => {
                    let input: UrlInput = serde_json::from_value(wavefront.input.clone())?;
                    self.handle_url(id, input, link)?;
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
    Box::new(BurnerPrism::new())
}
