//! Curl prism implementation for the Ultraviolet system.
//!
//! This crate provides a curl-based HTTP client prism that makes requests
//! with Amazon internal authentication.

pub mod spectrum;

use serde_json::json;
use spectrum::{FavoriteAccount, FavoritesResponse};
use uuid::Uuid;

use uv_core::{
    Result, UVError, UVLink, UVPrism, PrismMultiplexer, UVPulse, UVSpectrum
};

use crate::spectrum::HttpResponse;

/// Curl prism for making HTTP requests with Amazon internal auth.
pub struct AccountPrism {
    spectrum: Option<UVSpectrum>,
    multiplexer: PrismMultiplexer,
}

impl AccountPrism {
    /// Create a new curl prism.
    pub fn new() -> Self {
        Self {
            spectrum: None,
            multiplexer: PrismMultiplexer::new()
        }
    }
    
    fn build_console_url(account: &FavoriteAccount) -> String {
        format!(
            "https://iad.merlon.amazon.dev/console?awsAccountId={}&awsPartition={}&accountName={}&sessionDuration={}&policy=arn:aws:iam::aws:policy/{}",
            account.account_id,
            account.partition,
            account.account_name,
            account.session_duration,
            account.policy
        )
    }

    /// Handle GET requests.
    fn handle_list(&self, id: Uuid, link: &UVLink) -> Result<()> {
        // Use the curl.get refraction to get burner accounts
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.get",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://conduit.security.a2z.com/api/accounts/favorites"
            })
        )?;

        // Parse response
        let favorites: FavoritesResponse = serde_json::from_str(&response.body)
            .map_err(|e| UVError::RefractionError(format!("Failed to parse response: {}", e)))?;
        
        for account in favorites.favorite_account_list {
            link.emit_photon(id, json!({
                "accountName": account.account_name,
                "accountId": account.account_id,
                "consoleUrl": Self::build_console_url(&account)
            }))?;
        }

        // Signal successful completion
        link.emit_trap(id, None)?;

        Ok(())
    }
}

impl UVPrism for AccountPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "list" => {
                    // Handle the GET request
                    self.handle_list(id, link)?;
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

#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(AccountPrism::new())
}
