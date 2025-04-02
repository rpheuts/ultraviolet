//! Prism interface for the Ultraviolet system.
//!
//! The UVPrism trait defines the interface for all prisms in the system.
//! It uses a handler-based approach where the prism implements handlers for specific pulse types.

use async_trait::async_trait;
use uuid::Uuid;

use crate::pulse::UVPulse;
use crate::error::Result;
use crate::link::UVLink;
use crate::spectrum::UVSpectrum;

/// Core trait for all prisms in the system.
#[async_trait]
pub trait UVPrism: Send + Sync {
    /// Initialize the prism with its spectrum.
    ///
    /// This method is called when the prism is first loaded, before any links are established.
    /// It should initialize the prism with its spectrum and perform any necessary setup.
    async fn init(&mut self, spectrum: UVSpectrum) -> Result<()>;
    
    /// Get the prism's spectrum.
    ///
    /// This method returns a reference to the prism's spectrum, which contains
    /// information about the prism's capabilities and dependencies.
    fn spectrum(&self) -> &UVSpectrum;
    
    /// Called when a link is established with the prism.
    ///
    /// This is a setup hook, not for processing. It should perform any setup that needs
    /// to happen when a link is established, such as registering callbacks or initializing
    /// state that depends on the link.
    async fn link_established(&mut self, _link: &UVLink) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }
    
    /// Handle any pulse received on the link.
    ///
    /// This method is called for each pulse received on the link. It should handle the pulse
    /// and return true if the pulse was handled, or false if it should be ignored.
    ///
    /// The default implementation handles nothing and returns false.
    async fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool>;
    
    /// Called when the prism is about to be terminated.
    ///
    /// This is a cleanup hook, not for processing. It should perform any cleanup that needs
    /// to happen when the prism is shutting down, such as releasing resources or saving state.
    async fn shutdown(&self) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }
}
