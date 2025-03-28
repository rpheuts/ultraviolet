use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Alignment {
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "right")]
    Right,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogSection {
    pub title: String,
    #[serde(flatten)]
    pub content: DialogContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DialogContent {
    #[serde(rename = "key_value")]
    KeyValue {
        items: Vec<KeyValueItem>,
    },
    #[serde(rename = "table")]
    Table {
        source: String,
        columns: Vec<ColumnConfig>,
    },
    #[serde(rename = "text")]
    Text {
        value: String,
    },
    #[serde(rename = "code")]
    Code {
        value: String,
        language: Option<String>,
        resolve: Option<bool>,
    },
    #[serde(rename = "link")]
    Link {
        text: String,
        url: String,
        resolve: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValueItem {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub style: KeyValueStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum KeyValueStyle {
    #[serde(rename = "normal")]
    #[default]
    Normal,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableStyle {
    Default,
    Dense,
}

impl Default for TableStyle {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DisplayType {
    #[serde(rename = "table")]
    Table {
        source: String,
        columns: Vec<ColumnConfig>,
        #[serde(default)]
        style: TableStyle,
        #[serde(default)]
        format: Option<String>,
    },
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "help")]
    Help {
        #[serde(default)]
        style: HelpStyle,
    },
    #[serde(rename = "stream")]
    Stream {
        #[serde(default)]
        stderr_prefix: Option<String>,
        #[serde(default)]
        color_stderr: Option<bool>,
    },
    #[serde(rename = "csv")]
    Csv {
        source: String,
        columns: Vec<ColumnConfig>,
    },
    #[serde(rename = "dialog")]
    Dialog {
        sections: Vec<DialogSection>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HelpStyle {
    #[serde(rename = "compact")]
    #[default]
    Compact,
    #[serde(rename = "detailed")]
    Detailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpData {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub methods: Vec<MethodHelp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodHelp {
    pub path: Vec<String>,
    pub description: String,
    pub args: Option<Value>,
    pub returns: Option<Value>,
    pub display: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnConfig {
    pub title: String,
    pub value: String,
    #[serde(default)]
    pub align: Alignment,
    #[serde(default)]
    pub width: Option<usize>,
    #[serde(default)]
    pub max_width: Option<usize>,
    #[serde(default, rename = "type")]
    pub type_: Option<String>,  // "link", "text" (default), etc.
    #[serde(default)]
    pub url_value: Option<String>,  // Field containing the URL when type="link"
}

#[derive(Debug, Clone, Copy)]
pub enum StreamType {
    Stdout,
    Stderr,
}

pub trait Renderer {
    fn render(&self, value: &Value, display: &DisplayType) -> Result<String>;
    
    // New method for stream rendering
    fn render_stream(&self, stream_type: StreamType, line: &str, display: &DisplayType) -> Result<String> {
        match stream_type {
            StreamType::Stdout => Ok(line.to_string()),
            StreamType::Stderr => {
                if let DisplayType::Stream { stderr_prefix, color_stderr: _ } = display {
                    let mut output = String::new();
                    if let Some(prefix) = stderr_prefix {
                        output.push_str(prefix);
                        output.push(' ');
                    }
                    output.push_str(line);
                    Ok(output)
                } else {
                    Ok(line.to_string())
                }
            }
        }
    }
}

impl DisplayType {
    pub fn from_display_config(config: &Value) -> Option<Self> {
        serde_json::from_value(config.clone()).ok()
    }

    pub fn from_schema(_schema: &Value) -> Self {
        // We no longer infer from schema, always use explicit display config
        DisplayType::Raw
    }
}
