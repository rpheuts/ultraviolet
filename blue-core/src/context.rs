use std::collections::HashMap;
use std::path::PathBuf;
use std::io::Write;
use libloading::{Library, Symbol};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::module::Module;
use crate::manifest::ModuleManifest;

type CreateModuleFn = unsafe fn() -> Box<dyn Module>;

pub struct LoadedModule {
    _lib: Library, // Keep library loaded
    module: Box<dyn Module>,
}

pub struct ModuleContext {
    loaded_modules: HashMap<String, LoadedModule>,
    lib_path: PathBuf,
}

impl ModuleContext {
    pub fn new(lib_path: PathBuf) -> Self {
        Self {
            loaded_modules: HashMap::new(),
            lib_path,
        }
    }

    pub fn call_module(&mut self, module_ref: &str, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        if !self.loaded_modules.contains_key(module_ref) {
            // Parse namespace:name format
            let (namespace, name) = module_ref
                .split_once(':')
                .ok_or_else(|| Error::Module("Invalid module format. Use namespace:name".to_string()))?;

            // Load manifest first to validate
            let manifest_path = self.lib_path.join(namespace).join(name).join("manifest.toml");
            let manifest = ModuleManifest::load(&manifest_path)
                .map_err(|e| Error::Module(format!("Failed to load manifest: {}", e)))?;

            // Verify namespace and name match manifest
            if manifest.module.namespace != namespace || manifest.module.name != name {
                return Err(Error::Module(format!(
                    "Module namespace/name mismatch. Expected {}/{}, got {}/{}",
                    namespace, name, manifest.module.namespace, manifest.module.name
                )));
            }

            // Load module from namespaced path
            let lib_path = self.lib_path.join(namespace).join(name).join("module.dylib");

            // Load module library
            unsafe {
                let lib = Library::new(&lib_path)
                    .map_err(|e| Error::Module(format!("Failed to load module library: {}", e)))?;

                let create_module: Symbol<CreateModuleFn> = lib
                    .get(b"create_module")
                    .map_err(|e| Error::Module(format!("Module {}:{} missing create_module: {}", namespace, name, e)))?;

                let mut module = create_module();
                let context = ModuleContext::new(self.lib_path.clone());
                module.set_context(context);

                self.loaded_modules.insert(module_ref.to_string(), LoadedModule {
                    _lib: lib,
                    module,
                });
            }
        }

        // Get mutable reference to module and call it
        let module = &mut self.loaded_modules.get_mut(module_ref).unwrap().module;
        module.call(path, args, stdout, stderr)
    }
}
