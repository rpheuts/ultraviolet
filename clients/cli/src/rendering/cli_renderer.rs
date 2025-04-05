//! CLI renderer for UI components
//!
//! This module provides a renderer for UI components that outputs to the terminal.

use colored::Colorize;
use std::io::Write;
use uv_ui::{UIComponent, PropertyValue, Card, Table, List, Text};

/// Renderer for CLI output
pub struct CliRenderer {
    /// Maximum width for tables
    max_width: usize,
}

impl CliRenderer {
    /// Create a new CLI renderer
    pub fn new() -> Self {
        Self {
            max_width: 120,
        }
    }
    
    /// Render a component to the given writer
    pub fn render(&self, component: &UIComponent, writer: &mut impl Write) -> std::io::Result<()> {
        match component {
            UIComponent::Card(card) => self.render_card(card, writer),
            UIComponent::Table(table) => self.render_table(table, writer),
            UIComponent::List(list) => self.render_list(list, writer),
            UIComponent::Text(text) => self.render_text(text, writer),
        }
    }
    
    /// Render a card component
    fn render_card(&self, card: &Card, writer: &mut impl Write) -> std::io::Result<()> {
        // Get the title from the card or infer one
        let title = card.title.as_deref().or_else(|| card.infer_title()).unwrap_or("");
        
        // Render the title
        if title.len() > 0 {
            writeln!(writer, "{}", title.green().bold())?;
            writeln!(writer, "{}", "═".repeat(title.len()).green())?;
        }

        // If there is only 1 item, just render it
        if card.properties.len() == 1 {
            self.render_property_value(&card.properties.values().last().unwrap(), writer)?;
            writeln!(writer)?;
            return Ok(());
        }
        
        // Render properties
        for (key, value) in &card.properties {
            write!(writer, "  {}: ", key.blue())?;
            self.render_property_value(value, writer)?;
            writeln!(writer)?;
        }
        
        Ok(())
    }
    
    /// Render a table component
    fn render_table(&self, table: &Table, writer: &mut impl Write) -> std::io::Result<()> {
        // If no columns or rows, nothing to render
        if table.columns.is_empty() || table.rows.is_empty() {
            return writeln!(writer, "(empty table)");
        }
        
        // Get column widths
        let mut widths: Vec<usize> = vec![0; table.columns.len()];
        
        // Calculate header widths
        for (i, col) in table.columns.iter().enumerate() {
            widths[i] = col.name.len();
        }
        
        // Calculate data widths
        for row in &table.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let cell_str = self.property_value_to_string(cell);
                    widths[i] = widths[i].max(cell_str.len());
                }
            }
        }
        
        // Adjust widths if total exceeds max_width
        let total_width: usize = widths.iter().sum::<usize>() + (3 * widths.len());
        if total_width > self.max_width {
            // Simple proportional adjustment for now
            let excess = total_width - self.max_width;
            let avg_reduction = excess / widths.len();
            
            for width in &mut widths {
                if *width > avg_reduction + 3 {
                    *width -= avg_reduction;
                }
            }
        }
        
        // Render header
        write!(writer, "  ")?;
        for (i, col) in table.columns.iter().enumerate() {
            let name = format!("{:width$}", col.name, width = widths[i]);
            write!(writer, "{} │ ", name.green().bold())?;
        }
        writeln!(writer)?;
        
        // Render separator
        write!(writer, "  ")?;
        for i in 0..table.columns.len() {
            write!(writer, "{}", "─".repeat(widths[i]))?;
            if i < table.columns.len() - 1 {
                write!(writer, "─┼─")?;
            } else {
                write!(writer, "──")?;
            }
        }
        writeln!(writer)?;
        
        // Render rows
        for row in &table.rows {
            write!(writer, "  ")?;
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let cell_str = self.property_value_to_string(cell);
                    let truncated = if cell_str.len() > widths[i] {
                        format!("{}...", &cell_str[0..widths[i]-3])
                    } else {
                        cell_str
                    };
                    
                    write!(writer, "{:width$} │ ", truncated, width = widths[i])?;
                }
            }
            writeln!(writer)?;
        }
        
        Ok(())
    }
    
    /// Render a list component
    fn render_list(&self, list: &List, writer: &mut impl Write) -> std::io::Result<()> {
        if list.items.is_empty() {
            return writeln!(writer, "(empty list)");
        }
        
        for (i, item) in list.items.iter().enumerate() {
            write!(writer, "  {}. ", (i + 1).to_string().yellow())?;
            self.render_property_value(item, writer)?;
            writeln!(writer)?;
        }
        
        Ok(())
    }
    
    /// Render a text component
    fn render_text(&self, text: &Text, writer: &mut impl Write) -> std::io::Result<()> {
        // Check if we should format as code block
        if let Some(format) = &text.format {
            if format == "json" {
                writeln!(writer, "{}", "```json".dimmed())?;
                // Simple pretty-printing for JSON
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text.content) {
                    if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                        for line in pretty.lines() {
                            writeln!(writer, "{}", line)?;
                        }
                    } else {
                        writeln!(writer, "{}", text.content)?;
                    }
                } else {
                    writeln!(writer, "{}", text.content)?;
                }
                writeln!(writer, "{}", "```".dimmed())?;
                return Ok(());
            }
            
            if format == "markdown" {
                // Very basic markdown rendering
                for line in text.content.lines() {
                    if line.starts_with("# ") {
                        writeln!(writer, "{}", line[2..].green().bold())?;
                    } else if line.starts_with("## ") {
                        writeln!(writer, "{}", line[3..].green())?;
                    } else if line.starts_with("- ") {
                        writeln!(writer, "  • {}", &line[2..])?;
                    } else {
                        writeln!(writer, "{}", line)?;
                    }
                }
                return Ok(());
            }
        }
        
        // Default text rendering
        writeln!(writer, "{}", text.content)
    }
    
    /// Render a property value
    fn render_property_value(&self, value: &PropertyValue, writer: &mut impl Write) -> std::io::Result<()> {
        match value {
            PropertyValue::Component(component) => {
                // For nested components, render them directly
                writeln!(writer)?;
                self.render(component, writer)
            },
            _ => {
                // For regular values, render as before
                let value_str = self.property_value_to_string(value);
                
                // Colorize based on data type
                match value {
                    PropertyValue::Boolean(true) => write!(writer, "{}", value_str.green()),
                    PropertyValue::Boolean(false) => write!(writer, "{}", value_str.red()),
                    PropertyValue::Number(_) => write!(writer, "{}", value_str.yellow()),
                    PropertyValue::Date(_) => write!(writer, "{}", value_str.cyan()),
                    PropertyValue::URL(_) => write!(writer, "{}", value_str.underline().blue()),
                    PropertyValue::FilePath(_) => write!(writer, "{}", value_str.underline()),
                    PropertyValue::Component(_) => write!(writer, "{}", value_str), // This line is for pattern exhaustiveness
                    PropertyValue::Text(_) => write!(writer, "{}", value_str),
                    PropertyValue::Duration(_) => write!(writer, "{}", value_str),
                    PropertyValue::Other(_) => write!(writer, "{}", value_str),
                }
            }
        }
    }
    
    /// Convert a property value to a string
    fn property_value_to_string(&self, value: &PropertyValue) -> String {
        match value {
            PropertyValue::Text(s) => s.clone(),
            PropertyValue::Number(n) => n.to_string(),
            PropertyValue::Boolean(b) => b.to_string(),
            PropertyValue::Date(dt) => dt.to_rfc3339(),
            PropertyValue::Duration(d) => format_duration(*d),
            PropertyValue::URL(url) => url.clone(),
            PropertyValue::FilePath(path) => path.to_string_lossy().to_string(),
            PropertyValue::Component(_) => "[nested component]".to_string(),
            PropertyValue::Other(val) => match val {
                serde_json::Value::Null => "null".to_string(),
                _ => val.to_string(),
            },
        }
    }
}

/// Format a duration as a human-readable string
fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    
    if total_secs == 0 {
        return format!("{}ms", duration.subsec_millis());
    }
    
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    
    let mut parts = Vec::new();
    
    if days > 0 {
        parts.push(format!("{}d", days));
    }
    
    if hours > 0 || !parts.is_empty() {
        parts.push(format!("{}h", hours));
    }
    
    if minutes > 0 || !parts.is_empty() {
        parts.push(format!("{}m", minutes));
    }
    
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{}s", seconds));
    }
    
    parts.join(" ")
}
