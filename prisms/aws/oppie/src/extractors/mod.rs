//! Extractors for the oppie prism.
//!
//! This module provides extractors for various services.

use serde_json::Value;
use uv_core::Result;
use crate::writer::ExtractorWriter;

mod sas;
mod shepherd;
mod policy_engine;
mod cti;
mod org;
mod fua;
mod asr;

pub use sas::SasExtractor;
pub use shepherd::ShepherdExtractor;
pub use policy_engine::PolicyEngineExtractor;
pub use cti::CtiExtractor;
pub use org::OrgExtractor;
pub use fua::FuaExtractor;
pub use asr::AsrExtractor;

/// Trait for service data extractors.
pub trait Extractor {
    /// Process a user and extract data.
    fn process_user<W: ExtractorWriter>(&self, user: &str, writer: &mut W) -> Result<Value>;
}
