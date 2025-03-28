use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub module: ModuleInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub methods: Vec<MethodInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub namespace: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub path: Vec<String>,
    pub description: String,
    pub args_schema: Option<Value>,
    pub return_schema: Option<Value>,
    pub display: Option<Value>,
    _non_exhaustive: (),
}

impl MethodInfo {
    fn parse_json_field(field: &Option<String>) -> Option<Value> {
        field.as_ref().and_then(|s| serde_json::from_str(s).ok())
    }
}

#[derive(Deserialize)]
#[serde(rename = "MethodInfo")]
struct MethodInfoHelper {
    path: Vec<String>,
    description: String,
    #[serde(default)]
    args_schema: Option<String>,
    #[serde(default)]
    return_schema: Option<String>,
    #[serde(default)]
    display: Option<String>,
}

impl<'de> Deserialize<'de> for MethodInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = MethodInfoHelper::deserialize(deserializer)?;
        Ok(MethodInfo {
            path: helper.path,
            description: helper.description,
            args_schema: Self::parse_json_field(&helper.args_schema),
            return_schema: Self::parse_json_field(&helper.return_schema),
            display: Self::parse_json_field(&helper.display),
            _non_exhaustive: (),
        })
    }
}

impl Serialize for MethodInfo {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("MethodInfo", 5)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("args_schema", &self.args_schema)?;
        state.serialize_field("return_schema", &self.return_schema)?;
        state.serialize_field("display", &self.display)?;
        state.end()
    }
}

impl ModuleManifest {
    pub fn load(path: impl AsRef<std::path::Path>) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| crate::error::Error::Manifest(format!("Failed to read manifest: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| crate::error::Error::Manifest(format!("Failed to parse manifest: {}", e)))
    }

    pub fn find_method(&self, path: &[&str]) -> Option<&MethodInfo> {
        self.methods.iter().find(|method| {
            method.path.iter().map(String::as_str).collect::<Vec<_>>() == path
        })
    }
}
