use serde_json::Value;
use std::io::Write;
use crate::error::{Error, Result};
use crate::manifest::ModuleManifest;
use crate::context::ModuleContext;

/// Core interface for all modules
pub trait Module: Send + Sync {
    /// Get the name of the module
    fn name(&self) -> &str;

    /// Get the module's manifest
    fn manifest(&self) -> &ModuleManifest;

    /// Call a method on this module
    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value>;

    /// Set the module context
    fn set_context(&mut self, _context: ModuleContext) {
        // Default no-op implementation
    }
}

/// Basic implementation of a module that validates against its manifest
pub struct BaseModule {
    manifest: ModuleManifest,
    handler: Box<dyn Fn(&[&str], Value, Option<&mut dyn Write>, Option<&mut dyn Write>) -> Result<Value> + Send + Sync>,
    context: Option<ModuleContext>,
}

impl BaseModule {
    pub fn new(
        manifest: ModuleManifest,
        handler: impl Fn(&[&str], Value, Option<&mut dyn Write>, Option<&mut dyn Write>) -> Result<Value> + Send + Sync + 'static,
    ) -> Self {
        Self {
            manifest,
            handler: Box::new(handler),
            context: None,
        }
    }
}

impl Module for BaseModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Validate the method exists
        if self.manifest.find_method(path).is_none() {
            return Err(Error::MethodNotFound(path.join(" ").to_string()));
        }

        // Call the handler
        (self.handler)(path, args, stdout, stderr)
    }

    fn set_context(&mut self, context: ModuleContext) {
        self.context = Some(context);
    }
}
