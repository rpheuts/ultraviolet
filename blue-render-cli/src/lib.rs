use anyhow::{Result, anyhow};
use ansi_term::Colour::{Blue, Yellow, Cyan};
use ansi_term::Style;
use blue_render_core::{Renderer, DisplayType, HelpData, HelpStyle};
use serde_json::Value;

mod table;
use table::{Table, Cell, Alignment};

pub struct CliRenderer;

impl CliRenderer {
    pub fn new() -> Self {
        Self
    }

    fn resolve_value<'a>(&self, item: &'a Value, path: &str) -> &'a Value {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = item;
        for part in parts {
            current = current.get(part).unwrap_or(&Value::Null);
        }
        current
    }

    fn render_csv(&self, value: &Value, source: &str, columns: &[blue_render_core::ColumnConfig]) -> Result<String> {
        // Extract data using source path
        let array = if source.is_empty() {
            value.as_array()
                .ok_or_else(|| anyhow!("Expected array value for CSV display"))?
        } else {
            value.get(source)
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow!("Source '{}' not found or not an array", source))?
        };

        let mut output = String::new();
        
        // Add header row
        output.push_str(&columns.iter()
            .map(|col| format!("\"{}\"", col.title.replace("\"", "\"\"")))
            .collect::<Vec<_>>()
            .join(","));
        output.push('\n');

        // Add data rows
        for item in array {
            let row = columns.iter()
                .map(|col| {
                    let value = self.resolve_value(item, &col.value);
                    let text = match value {
                        Value::Null => String::new(),
                        Value::Bool(b) => b.to_string(),
                        Value::Number(n) => n.to_string(),
                        Value::String(s) => s.clone(),
                        _ => serde_json::to_string(value).unwrap_or_default(),
                    };
                    format!("\"{}\"", text.replace("\"", "\"\""))
                })
                .collect::<Vec<_>>()
                .join(",");
            output.push_str(&row);
            output.push('\n');
        }

        Ok(output)
    }

    fn render_help_compact(&self, help: &HelpData) -> Result<String> {
        let mut output = format!("\n{} {}\n", 
            Blue.bold().paint(&help.name),
            Style::new().dimmed().paint(format!("v{}", help.version))
        );
        
        if let Some(desc) = &help.description {
            output.push_str(&format!("{}\n", desc));
        }

        for method in &help.methods {
            output.push_str(&format!("\n{} {}\n", 
                Blue.bold().paint("Method:"),
                Cyan.paint(method.path.join(" "))
            ));
            output.push_str(&format!("{}\n", method.description));
        }

        Ok(output)
    }

    fn render_help_detailed(&self, help: &HelpData) -> Result<String> {
        let mut output = format!("\n{} {}\n", 
            Blue.bold().paint(&help.name),
            Style::new().dimmed().paint(format!("v{}", help.version))
        );
        
        if let Some(desc) = &help.description {
            output.push_str(&format!("{}\n", desc));
        }

        for method in &help.methods {
            output.push_str(&format!("\n{} {}\n", 
                Blue.bold().paint("Method:"),
                Cyan.paint(method.path.join(" "))
            ));
            output.push_str(&format!("{}\n", method.description));

            if let Some(args) = &method.args {
                output.push_str(&format!("\n{}\n", Yellow.paint("Arguments:")));
                
                // Show required args first
                if let Some(required) = args.get("required") {
                    for field in required.as_array().unwrap() {
                        output.push_str(&format!("  {} {}\n",
                            Yellow.paint(format!("--{}", field.as_str().unwrap())),
                            Style::new().dimmed().paint("(Required)")
                        ));
                    }
                }

                // Then show optional args
                if let Some(props) = args.get("properties") {
                    for (name, schema) in props.as_object().unwrap() {
                        let desc = schema.get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if !desc.is_empty() {
                            output.push_str(&format!("  {} {}\n",
                                Yellow.paint(format!("--{}", name)),
                                Style::new().dimmed().paint(desc)
                            ));
                        }
                    }
                }
            }

            if let Some(returns) = &method.returns {
                output.push_str(&format!("\n{}\n", Yellow.paint("Returns:")));
                if let Some(props) = returns.get("properties") {
                    for (name, schema) in props.as_object().unwrap() {
                        let type_str = schema.get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("any");
                        output.push_str(&format!("  {} {}\n",
                            Yellow.paint(name),
                            Style::new().dimmed().paint(type_str)
                        ));
                    }
                }
            }

            output.push_str("\n");
        }

        Ok(output)
    }

    fn render_table(&self, value: &Value, source: &str, columns: &[blue_render_core::ColumnConfig], table_config: Option<&Table>) -> Result<String> {
        // Extract data using source path
        let array = if source.is_empty() {
            value.as_array()
                .ok_or_else(|| anyhow!("Expected array value for table display"))?
        } else {
            // Resolve the source path first
            let resolved = self.resolve_value(value, source);
            resolved.as_array()
                .ok_or_else(|| anyhow!("Source '{}' not found or not an array", source))?
        };

        // Create table or use provided one
        let mut table = table_config.cloned().unwrap_or_else(Table::new);

        // Add header row
        let headers: Vec<Cell> = columns.iter()
            .map(|col| Cell::Text(Blue.bold().paint(&col.title).to_string()))
            .collect();
        table.set_headers(headers);

        // Set column alignments and max widths
        for (i, col) in columns.iter().enumerate() {
            let alignment = match col.align {
                blue_render_core::Alignment::Left => Alignment::Left,
                blue_render_core::Alignment::Center => Alignment::Center,
                blue_render_core::Alignment::Right => Alignment::Right,
            };
            table.set_column_alignment(i, alignment);
            table.set_column_max_width(i, col.max_width);
        }

        // Add data rows
        for item in array {
            let row: Vec<Cell> = columns.iter()
                .map(|col| {
                    let value = self.resolve_value(item, &col.value);
                    if col.type_.as_deref() == Some("link") {
                        // Get the display text
                        let text = match value {
                            Value::String(s) => s.clone(),
                            _ => serde_json::to_string(value).unwrap_or_default(),
                        };

                        // Get the URL and create the link
                        if let Some(url_field) = &col.url_value {
                            if let Some(url) = self.resolve_value(item, url_field).as_str() {
                                Cell::Link {
                                    text,
                                    url: url.replace(" ", "%20"),
                                }
                            } else {
                                Cell::Text(text)
                            }
                        } else {
                            Cell::Text(text)
                        }
                    } else {
                        // Regular text handling
                        let text = match value {
                            Value::Null => String::new(),
                            Value::Bool(b) => b.to_string(),
                            Value::Number(n) => n.to_string(),
                            Value::String(s) => s.clone(),
                            _ => serde_json::to_string(value).unwrap_or_default(),
                        };
                        Cell::Text(text)
                    }
                })
                .collect();
            table.add_row(row);
        }

        Ok(table.render())
    }

    fn render_text(&self, value: &Value) -> Result<String> {
        // For objects with a message field, use that
        if let Some(obj) = value.as_object() {
            if let Some(msg) = obj.get("message").and_then(|v| v.as_str()) {
                return Ok(msg.to_string());
            }
            // If no message field but response is from burner API
            if obj.contains_key("success") {
                let success = obj.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                let status = if success { "Success" } else { "Error" };
                let message = obj.get("message").and_then(|v| v.as_str()).unwrap_or("No message provided");
                return Ok(format!("{}: {}", status, message));
            }
        }
        
        // Try as direct string
        if let Some(s) = value.as_str() {
            return Ok(s.to_string());
        }

        // If all else fails, use pretty JSON
        Ok(serde_json::to_string_pretty(value)?)
    }

    fn render_key_value(&self, items: &[blue_render_core::KeyValueItem], root_value: &Value) -> Result<String> {
        let mut output = String::new();
        
        for item in items {
            let style = match item.style {
                blue_render_core::KeyValueStyle::Normal => Style::new(),
                blue_render_core::KeyValueStyle::Success => Style::new().fg(ansi_term::Colour::Green),
                blue_render_core::KeyValueStyle::Warning => Style::new().fg(ansi_term::Colour::Yellow),
                blue_render_core::KeyValueStyle::Error => Style::new().fg(ansi_term::Colour::Red),
            };
            
            // Resolve the value from the root object
            let resolved_value = self.resolve_value(root_value, &item.value);
            let display_value = match resolved_value {
                Value::Null => String::new(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                _ => serde_json::to_string(resolved_value).unwrap_or_default(),
            };
            
            output.push_str(&format!("{}: {}\n",
                Yellow.paint(&item.key),
                style.paint(&display_value)
            ));
        }

        Ok(output)
    }

    fn render_code(&self, value: &str, language: Option<&str>, resolve: bool, item: Option<&Value>) -> Result<String> {
        let mut output = String::new();
        
        if let Some(lang) = language {
            output.push_str(&format!("Language: {}\n", Blue.paint(lang)));
        }
        
        // Add a line before and after the code block
        output.push_str("─".repeat(80).as_str());
        output.push('\n');

        // If resolve is true and we have an item, resolve the value path
        let content = if resolve {
            if let Some(obj) = item {
                let resolved = self.resolve_value(obj, value);
                serde_json::to_string_pretty(resolved)?
            } else {
                value.to_string()
            }
        } else {
            value.to_string()
        };

        output.push_str(&content);
        if !content.ends_with('\n') {
            output.push('\n');
        }
        output.push_str("─".repeat(80).as_str());
        output.push('\n');

        Ok(output)
    }

    fn render_dialog_section(&self, section: &blue_render_core::DialogSection, root_value: &Value) -> Result<String> {
        let mut output = String::new();

        // Add section header if title is not empty
        if !section.title.is_empty() {
            output.push_str(&format!("\n{}\n",
                Blue.bold().paint(&section.title)
            ));
            output.push_str(&"═".repeat(section.title.len()));
            output.push_str("\n\n");
        }

        // Render section content
        let content = match &section.content {
            blue_render_core::DialogContent::KeyValue { items } => {
                self.render_key_value(items, root_value)?
            }
            blue_render_core::DialogContent::Table { source, columns } => {
                self.render_table(root_value, source, columns, None)?
            }
            blue_render_core::DialogContent::Text { value } => {
                format!("{}\n", value)
            }
            blue_render_core::DialogContent::Code { value, language, resolve } => {
                self.render_code(value, language.as_deref(), resolve.unwrap_or(false), Some(root_value))?
            }
            blue_render_core::DialogContent::Link { text, url, resolve } => {
                let url_value = if resolve.unwrap_or(false) {
                    self.resolve_value(root_value, url)
                        .as_str()
                        .unwrap_or(url)
                        .to_string()
                } else {
                    url.to_string()
                };
                format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", url_value, Blue.paint(text))
            }
        };

        output.push_str(&content);
        output.push('\n');

        Ok(output)
    }
}

impl Renderer for CliRenderer {
    fn render(&self, value: &Value, display: &DisplayType) -> Result<String> {
        match display {
            DisplayType::Table { source, columns, style, format: _ } => {
                let mut table = Table::new();
                table.set_style(style.clone());
                self.render_table(value, source, columns, Some(&table))
            }
            DisplayType::Text => {
                self.render_text(value)
            }
            DisplayType::Raw => {
                Ok(serde_json::to_string_pretty(value)?)
            }
            DisplayType::Stream { .. } => {
                // For stream type, we expect the value to be already handled by the streaming client
                Ok(String::new())
            }
            DisplayType::Help { style } => {
                let help: HelpData = serde_json::from_value(value.clone())?;
                match style {
                    HelpStyle::Compact => self.render_help_compact(&help),
                    HelpStyle::Detailed => self.render_help_detailed(&help),
                }
            }
            DisplayType::Csv { source, columns } => {
                self.render_csv(value, source, columns)
            }
            DisplayType::Dialog { sections } => {
                let mut output = String::new();
                for section in sections {
                    output.push_str(&self.render_dialog_section(section, value)?);
                }
                Ok(output)
            }
        }
    }
}
