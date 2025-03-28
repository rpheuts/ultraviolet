use anyhow::{Result, anyhow};
use std::path::PathBuf;

pub struct NamespaceResolver {
    lib_path: PathBuf,
}

impl NamespaceResolver {
    pub fn new(lib_path: PathBuf) -> Self {
        Self { lib_path }
    }

    // Try to resolve a module name to namespace:module format
    pub fn resolve(&self, module: &str) -> Result<String> {
        // If already has namespace, return as-is
        if module.contains(':') {
            return Ok(module.to_string());
        }

        let mut matches = Vec::new();
        
        // Search all namespaces for the module
        for namespace_entry in std::fs::read_dir(&self.lib_path)? {
            let namespace = namespace_entry?.file_name();
            let module_path = self.lib_path.join(&namespace).join(module);
            
            if module_path.is_dir() {
                matches.push(format!("{}:{}", namespace.to_str().unwrap(), module));
            }
        }

        match matches.len() {
            0 => Err(anyhow!("Module '{}' not found", module)),
            1 => Ok(matches[0].clone()),
            _ => Err(anyhow!(
                "Ambiguous module name '{}'. Found in multiple namespaces: {}",
                module,
                matches.join(", ")
            ))
        }
    }
}
