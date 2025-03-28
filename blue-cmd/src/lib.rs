use blue_core::prelude::*;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct CmdModule {
    manifest: ModuleManifest,
    context: Option<ModuleContext>,
}

impl CmdModule {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;
        let lib_path = home.join(".blue/modules");
        let manifest = ModuleManifest::load(lib_path.join("blue").join("cmd").join("manifest.toml"))?;
        Ok(Self {
            manifest,
            context: None,
        })
    }

    fn handle_script(&mut self, args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Missing required argument: path".into()))?;

        // Expand path (handle ~ etc.)
        let expanded_path = shellexpand::full(path)
            .map_err(|e| Error::Module(format!("Failed to expand path: {}", e)))?;

        // Verify script exists and is readable
        let script_path = PathBuf::from(expanded_path.as_ref());
        if !script_path.exists() {
            return Err(Error::Module(format!("Script not found: {}", path)));
        }

        // Get optional arguments
        let script_args = args.get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>())
            .unwrap_or_default();

        let shell = args.get("shell")
            .and_then(|v| v.as_str())
            .unwrap_or("bash");

        let env_vars = args.get("env")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<serde_json::Map<String, Value>>())
            .unwrap_or_default();

        // Build command that executes the script with its arguments
        let mut command = format!("{} ", script_path.display());
        for arg in script_args {
            command.push_str(&shell_escape::escape(arg.into()));
            command.push(' ');
        }

        // Call core-cmd exec
        let context = self.context.as_mut()
            .ok_or_else(|| Error::Module("No module context available".into()))?;

        context.call_module("blue:core-cmd", &["exec"], json!({
            "command": command,
            "shell": shell,
            "env": env_vars
        }), stdout, stderr)
    }

    fn handle_run(&mut self, args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Missing required argument: command".into()))?;

        let env_vars = args.get("env")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<serde_json::Map<String, Value>>())
            .unwrap_or_default();

        // Call core-cmd exec
        let context = self.context.as_mut()
            .ok_or_else(|| Error::Module("No module context available".into()))?;

        context.call_module("blue:core-cmd", &["exec"], json!({
            "command": command,
            "env": env_vars
        }), stdout, stderr)
    }
}

impl Module for CmdModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn set_context(&mut self, context: ModuleContext) {
        self.context = Some(context);
    }

    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Validate method exists
        if self.manifest.find_method(path).is_none() {
            return Err(Error::MethodNotFound(path.join(" ")));
        }

        match path {
            ["script"] => self.handle_script(args, stdout, stderr),
            ["run"] => self.handle_run(args, stdout, stderr),
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(CmdModule::new().expect("Failed to create cmd module"))
}
