//! Core types and functionality for the Ultraviolet system.
//!
//! This crate provides the fundamental types and abstractions used throughout
//! the Ultraviolet system, including pulse protocol definitions, spectrum formats,
//! and refraction mechanisms.

mod pulse;
mod error;
mod spectrum;
mod refraction;
mod transport;
mod link;
mod prism;
mod prism_core;
mod multiplexer;

// Re-export core types
pub use pulse::{UVPulse, Wavefront, Photon, Trap};
pub use error::{UVError, Result};
pub use spectrum::{UVSpectrum, Wavelength, SchemaDefinition};
pub use refraction::{Refraction, PropertyMapper};
pub use transport::{Transport, create_transport_pair};
pub use link::UVLink;
pub use prism::UVPrism;
pub use prism_core::UVPrismCore;
pub use multiplexer::PrismMultiplexer;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
