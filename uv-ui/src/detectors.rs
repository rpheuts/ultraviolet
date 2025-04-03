//! Type detectors for automatically identifying data types

use std::path::PathBuf;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde_json::Value;
use url::Url;

use crate::{TypeDetector, PropertyValue};

/// Collection of all available type detectors
pub struct TypeDetectors {
    detectors: Vec<Box<dyn TypeDetector>>,
}

impl TypeDetectors {
    /// Create a new collection with all default detectors
    pub fn new() -> Self {
        let mut detectors: Vec<Box<dyn TypeDetector>> = Vec::new();
        
        // Add all detectors in priority order (more specific first)
        detectors.push(Box::new(BooleanDetector));
        detectors.push(Box::new(NumberDetector));
        detectors.push(Box::new(DateTimeDetector));
        detectors.push(Box::new(DurationDetector));
        detectors.push(Box::new(URLDetector));
        detectors.push(Box::new(FilePathDetector));
        detectors.push(Box::new(TextDetector)); // Most general, should be last
        
        Self { detectors }
    }
    
    /// Detect the type of a value using all detectors
    pub fn detect(&self, value: &Value) -> PropertyValue {
        // Try each detector in order
        for detector in &self.detectors {
            if let Some(property) = detector.detect(value) {
                return property;
            }
        }
        
        // Default to raw Value if no detector matches
        PropertyValue::Other(value.clone())
    }
}

/// Detector for boolean values
pub struct BooleanDetector;

impl TypeDetector for BooleanDetector {
    fn detect(&self, value: &Value) -> Option<PropertyValue> {
        match value {
            Value::Bool(b) => Some(PropertyValue::Boolean(*b)),
            Value::String(s) => {
                let lower = s.to_lowercase();
                if lower == "true" || lower == "false" || 
                   lower == "yes" || lower == "no" ||
                   lower == "y" || lower == "n" {
                    let is_true = lower == "true" || lower == "yes" || lower == "y";
                    Some(PropertyValue::Boolean(is_true))
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}

/// Detector for numeric values
pub struct NumberDetector;

impl TypeDetector for NumberDetector {
    fn detect(&self, value: &Value) -> Option<PropertyValue> {
        match value {
            Value::Number(n) => n.as_f64().map(PropertyValue::Number),
            Value::String(s) => {
                // Try to parse as a number
                s.parse::<f64>().ok().map(PropertyValue::Number)
            },
            _ => None,
        }
    }
}

/// Detector for URL values
pub struct URLDetector;

impl TypeDetector for URLDetector {
    fn detect(&self, value: &Value) -> Option<PropertyValue> {
        if let Value::String(s) = value {
            // Check if it's a valid URL
            if s.starts_with("http://") || s.starts_with("https://") || 
               s.starts_with("ftp://") || s.starts_with("file://") {
                if Url::parse(s).is_ok() {
                    return Some(PropertyValue::URL(s.clone()));
                }
            }
        }
        None
    }
}

/// Detector for file path values
pub struct FilePathDetector;

impl TypeDetector for FilePathDetector {
    fn detect(&self, value: &Value) -> Option<PropertyValue> {
        if let Value::String(s) = value {
            // Check if it looks like a file path
            // This is a basic check, could be improved
            if s.contains('/') || s.contains('\\') {
                return Some(PropertyValue::FilePath(PathBuf::from(s)));
            }
        }
        None
    }
}

/// Detector for date/time values
pub struct DateTimeDetector;

impl TypeDetector for DateTimeDetector {
    fn detect(&self, value: &Value) -> Option<PropertyValue> {
        if let Value::String(s) = value {
            // Try common date formats
            
            // ISO 8601 / RFC 3339
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Some(PropertyValue::Date(dt.with_timezone(&Utc)));
            }
            
            // RFC 2822
            if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
                return Some(PropertyValue::Date(dt.with_timezone(&Utc)));
            }
            
            // Common date formats
            let formats = [
                "%Y-%m-%d",              // 2025-01-01
                "%Y-%m-%d %H:%M:%S",     // 2025-01-01 13:45:30
                "%Y/%m/%d",              // 2025/01/01
                "%d/%m/%Y",              // 01/01/2025
                "%m/%d/%Y",              // 01/01/2025
                "%b %d, %Y",             // Jan 01, 2025
                "%B %d, %Y",             // January 01, 2025
                "%Y-%m-%dT%H:%M:%S",     // 2025-01-01T13:45:30
            ];
            
            for format in &formats {
                if let Ok(dt) = DateTime::parse_from_str(s, format) {
                    return Some(PropertyValue::Date(dt.into()));
                }
            }
        }
        None
    }
}

/// Detector for duration values
pub struct DurationDetector;

impl TypeDetector for DurationDetector {
    fn detect(&self, value: &Value) -> Option<PropertyValue> {
        match value {
            Value::Number(n) => {
                // Assume number is in seconds
                if let Some(secs) = n.as_f64() {
                    let duration = Duration::from_secs_f64(secs);
                    return Some(PropertyValue::Duration(duration));
                }
            },
            Value::String(s) => {
                // Try to parse a duration string like "1h 30m" or "90s"
                if let Some(duration) = parse_duration(s) {
                    return Some(PropertyValue::Duration(duration));
                }
            },
            _ => (),
        }
        None
    }
}

/// Detector for text values (fallback)
pub struct TextDetector;

impl TypeDetector for TextDetector {
    fn detect(&self, value: &Value) -> Option<PropertyValue> {
        match value {
            Value::String(s) => Some(PropertyValue::Text(s.clone())),
            _ => None,
        }
    }
}

// Helper for parsing duration strings like "1h 30m" or "90s"
fn parse_duration(s: &str) -> Option<Duration> {
    // Simple duration parser for common formats
    let s = s.trim().to_lowercase();
    
    // Check for specific patterns
    
    // Format: "1h 30m 45s"
    if s.contains(' ') {
        let mut total_secs = 0.0;
        let parts: Vec<&str> = s.split(' ').collect();
        
        for part in parts {
            if let Some(d) = parse_single_duration(part) {
                total_secs += d.as_secs_f64();
            } else {
                return None; // Invalid part
            }
        }
        
        return Some(Duration::from_secs_f64(total_secs));
    }
    
    // Format: "1h30m45s"
    let mut remaining = s.clone();
    let mut total_secs = 0.0;
    
    // Extract hours if present
    if let Some(pos) = remaining.find('h') {
        let hours = &remaining[0..pos];
        if let Ok(h) = hours.parse::<f64>() {
            total_secs += h * 3600.0;
            remaining = (&remaining[pos+1..]).to_string();
        } else {
            return None; // Invalid hours
        }
    }
    
    // Extract minutes if present
    if let Some(pos) = remaining.find('m') {
        let minutes = &remaining[0..pos];
        if let Ok(m) = minutes.parse::<f64>() {
            total_secs += m * 60.0;
            remaining = (&remaining[pos+1..]).to_string();
        } else {
            return None; // Invalid minutes
        }
    }
    
    // Extract seconds if present
    if let Some(pos) = remaining.find('s') {
        let seconds = &remaining[0..pos];
        if let Ok(s) = seconds.parse::<f64>() {
            total_secs += s;
            remaining = (&remaining[pos+1..]).to_string();
        } else {
            return None; // Invalid seconds
        }
    }
    
    // Check if there's any unparsed content
    if !remaining.is_empty() {
        return None;
    }
    
    // If we parsed at least something
    if total_secs > 0.0 {
        Some(Duration::from_secs_f64(total_secs))
    } else {
        // Try as a simple number of seconds
        s.parse::<f64>().ok().map(Duration::from_secs_f64)
    }
}

fn parse_single_duration(s: &str) -> Option<Duration> {
    if s.is_empty() {
        return None;
    }
    
    let last_char = s.chars().last().unwrap();
    let value_str = &s[0..s.len()-1];
    
    if let Ok(value) = value_str.parse::<f64>() {
        match last_char {
            'h' => return Some(Duration::from_secs_f64(value * 3600.0)),
            'm' => return Some(Duration::from_secs_f64(value * 60.0)),
            's' => return Some(Duration::from_secs_f64(value)),
            'd' => return Some(Duration::from_secs_f64(value * 86400.0)),
            'w' => return Some(Duration::from_secs_f64(value * 604800.0)),
            _ => (),
        }
    }
    
    None
}