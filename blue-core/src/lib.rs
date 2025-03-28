mod error;
mod manifest;
pub mod module;
mod streaming;
pub mod context;

pub use error::{Error, Result};
pub use manifest::{ModuleManifest, ModuleInfo, MethodInfo};
pub use module::{Module, BaseModule};
pub use streaming::StreamWriter;
pub use std::io::Write;
pub use context::{ModuleContext, LoadedModule};

/// Re-export common types used in method calls
pub use serde_json::Value;

/// Prelude module for commonly used types
pub mod prelude {
    pub use crate::{
        Error,
        Result,
        Module,
        ModuleManifest,
        ModuleInfo,
        MethodInfo,
        ModuleContext,
        Value,
        Write,
        StreamWriter
    };
}
