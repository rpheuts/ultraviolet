use anyhow::{Context, Result};
use serde_json::{json, Value};
use uuid::Uuid;
use uv_core::{PrismMultiplexer, UVSpectrum};

use crate::parsing::cli_args::parse_args_to_map;
use crate::local::process_response;

/// Fetch the spectrum for a remote prism
fn fetch_remote_spectrum(remote_url: &str, prism_id: &str) -> Result<UVSpectrum> {
    let multiplexer = PrismMultiplexer::new();
    let spectrum_link = multiplexer.establish_link("system:remote")?;
    
    // Create a request to get the spectrum
    let describe_request = json!({
        "url": remote_url,
        "prism": "system:discovery",
        "frequency": "describe",
        "input": {
            "prismId": prism_id
        }
    });
    
    // Send the request and get spectrum as JSON string
    spectrum_link.send_wavefront(Uuid::new_v4(), "system:remote", "refract", describe_request)?;
    let spectrum_value = spectrum_link.absorb::<Value>()?;
    let spectrum_json = serde_json::to_string(&spectrum_value)?;
    
    // Parse using the constructor
    Ok(UVSpectrum::new_from_json(spectrum_json)?)
}

/// Resolve a prism name to a fully qualified prism ID with namespace
fn resolve_prism_name(remote_url: &str, prism_name: &str) -> Result<String> {
    if prism_name.contains(':') {
        // Already has a namespace
        return Ok(prism_name.to_string());
    }
    
    // Get list of remote prisms
    let multiplexer = PrismMultiplexer::new();
    let bridge_link = multiplexer.establish_link("system:remote")?;
    
    // Create a request to the remote discovery prism
    let request = json!({
        "url": remote_url,
        "prism": "system:discovery",
        "frequency": "list",
        "input": {}
    });
    
    // Send the request through the bridge
    bridge_link.send_wavefront(Uuid::new_v4(), "system:remote", "refract", request)?;
    
    // Use the link to collect all prisms
    let prisms_value = bridge_link.absorb::<Vec<Value>>()?;
    
    // Find matches for the requested prism name
    let mut matches = Vec::new();
    for prism in prisms_value {
        if let (Some(name), Some(namespace)) = (
            prism.get("name").and_then(|v| v.as_str()),
            prism.get("namespace").and_then(|v| v.as_str())
        ) {
            if name == prism_name {
                matches.push(format!("{}:{}", namespace, name));
            }
        }
    }
    
    // Return the appropriate prism ID
    match matches.len() {
        0 => Err(anyhow::format_err!("No prism with name '{}' found on remote server", prism_name)),
        1 => Ok(matches[0].clone()),
        _ => Err(anyhow::format_err!("Multiple prisms with name '{}' found. Please specify namespace. Available: {}", 
            prism_name, matches.join(", "))),
    }
}

/// Handle remote commands with enhanced features
pub fn handle_remote(remote_url: &str, prism_name: &str, args: Vec<String>, output: Option<&String>) -> Result<()> {
    // Extract frequency from args (first element)
    let (frequency, input_args) = args.split_first()
        .context("No frequency provided")?;
    
    // Parse the input arguments
    let input = serde_json::Value::Object(parse_args_to_map(input_args));
    
    // Try to resolve prism name if it doesn't have a namespace
    let resolved_prism = resolve_prism_name(remote_url, prism_name)?;
    
    // Fetch the spectrum for rendering
    let spectrum = fetch_remote_spectrum(remote_url, &resolved_prism)?;
    
    // Find the wavelength for the requested frequency
    let wavelength = spectrum.find_wavelength(frequency)
        .ok_or_else(|| anyhow::format_err!("No wavelength found for frequency '{}' in prism '{}'", frequency, resolved_prism))?;
    
    // Now make the actual request
    let multiplexer = PrismMultiplexer::new();
    let bridge_link = multiplexer.establish_link("system:remote")?;
    
    // Create the refract request
    let request = json!({
        "url": remote_url,
        "prism": resolved_prism,
        "frequency": frequency,
        "input": input
    });
    
    // Send the request
    bridge_link.send_wavefront(Uuid::new_v4(), "system:remote", "refract", request)?;
    
    // Process the response using the existing function from local.rs
    process_response(bridge_link, &wavelength.output, output)
}
