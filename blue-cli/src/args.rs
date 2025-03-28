use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashMap;

pub struct ArgProcessor {
    args_schema: Value,
    named_args: HashMap<String, Vec<String>>,
    positional_args: Vec<String>,
    flags: HashMap<String, bool>,
}

impl ArgProcessor {
    pub fn new(args_schema: Value) -> Self {
        Self {
            args_schema,
            named_args: HashMap::new(),
            positional_args: Vec::new(),
            flags: HashMap::new(),
        }
    }

    fn is_array_param(&self, name: &str) -> bool {
        // Check if this exact name is an array type
        if let Some("array") = self.get_param_type(name) {
            return true;
        }

        // If name doesn't end in 's', check if plural form is an array
        if !name.ends_with('s') {
            let plural = format!("{}s", name);
            if let Some("array") = self.get_param_type(&plural) {
                return true;
            }
        }
        // If name ends in 's', check if singular form is an array
        else if name.len() > 1 {
            let singular = &name[..name.len()-1];
            if let Some("array") = self.get_param_type(singular) {
                return true;
            }
        }

        false
    }

    pub fn is_boolean_param(&self, name: &str) -> bool {
        self.get_param_type(name) == Some("boolean")
    }

    fn get_array_name(&self, name: &str) -> String {
        // If name doesn't end in 's' and plural form exists as array
        if !name.ends_with('s') {
            let plural = format!("{}s", name);
            if let Some("array") = self.get_param_type(&plural) {
                return plural;
            }
        }
        // If name ends in 's' and singular form exists as array
        else if name.len() > 1 {
            let singular = &name[..name.len()-1];
            if let Some("array") = self.get_param_type(singular) {
                return singular.to_string();
            }
        }
        // Default to original name
        name.to_string()
    }

    pub fn add_named_arg(&mut self, name: &str, value: &str) {
        let array_name = self.get_array_name(name);
        self.named_args.entry(array_name)
            .or_insert_with(Vec::new)
            .push(value.to_string());
    }

    pub fn add_flag(&mut self, name: &str) {
        self.flags.insert(name.to_string(), true);
    }

    pub fn add_positional_arg(&mut self, value: &str) {
        self.positional_args.push(value.to_string());
    }

    fn get_param_type(&self, param_name: &str) -> Option<&str> {
        self.args_schema.get("properties")
            .and_then(|props| props.get(param_name))
            .and_then(|param| param.get("type"))
            .and_then(|t| t.as_str())
    }

    fn get_required_string_params(&self) -> Vec<String> {
        let mut required = Vec::new();
        
        if let Some(props) = self.args_schema.get("properties") {
            if let Some(required_array) = self.args_schema.get("required") {
                if let Some(required_params) = required_array.as_array() {
                    for param in required_params {
                        if let Some(param_name) = param.as_str() {
                            if let Some(param_schema) = props.get(param_name) {
                                if param_schema.get("type").and_then(|t| t.as_str()) == Some("string") {
                                    required.push(param_name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        required
    }

    pub fn process(&self) -> Result<Value> {
        let mut result = json!({});
        let obj = result.as_object_mut().unwrap();

        // Process named arguments according to their schema type
        for (name, values) in &self.named_args {
            if self.is_array_param(&name) {
                // For array types, include all values as an array
                obj.insert(name.clone(), json!(values));
            } else {
                // For non-array types, just use the first value
                if let Some(first) = values.first() {
                    obj.insert(name.clone(), json!(first));
                }
            }
        }

        // Process boolean flags
        for (name, value) in &self.flags {
            if self.is_boolean_param(&name) {
                obj.insert(name.clone(), json!(value));
            }
        }

        // Process positional arguments
        let required_params = self.get_required_string_params();
        let mut pos_idx = 0;

        for param in required_params {
            // Skip if already set by named argument
            if !obj.contains_key(&param) && pos_idx < self.positional_args.len() {
                obj.insert(param, json!(self.positional_args[pos_idx]));
                pos_idx += 1;
            }
        }

        // Validate against schema
        if let Some(required) = self.args_schema.get("required") {
            if let Some(required_params) = required.as_array() {
                for param in required_params {
                    if let Some(param_name) = param.as_str() {
                        if !obj.contains_key(param_name) {
                            anyhow::bail!("Missing required parameter: {}", param_name);
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}
