use anyhow::{Context, Result};
use serde_json::Value;
use uuid::Uuid;
use uv_core::{PrismMultiplexer, UVLink, UVSchemaDefinition, UVSpectrum};

use crate::{parsing::{cli_args::parse_args_to_map, cli_preprocessor::preprocess}, rendering::{cli_help::handle_help_request, cli_renderer::{render_array, render_object, render_stream}}};

pub fn handle_local(prism: &str, args: Vec<String>, output: Option<&String>) -> Result<()> {
    let (name, namespace) = UVSpectrum::resolve_prism_id(&prism.to_string())?;
    let prism_id = format!("{}:{}", namespace, name);

    if args.iter().any(|arg| arg == "--help" || arg == "-h") || prism == "help" {
        return handle_help_request(prism, &args);
    }

    // Find the spectrum for the given prism
    let spectrum = UVSpectrum::new(&prism_id)
        .context(format!("Unable to find spectrum for prism {}", prism_id))?;

    // Extract the frequency from the arguments
    let (frequency, args) = args.split_first()
        .context("No frequency provided")?;

    // Find the wavelength in the spectrum for the given prism
    let wavelength = spectrum.find_wavelength(frequency)
        .context(format!("Unable to find frequency {} on prism {}", frequency, prism_id))?;

    // Parse the remaining arguments into an input object
    let mut input = serde_json::Value::Object(parse_args_to_map(args));
    input = preprocess(input, &wavelength.input)?;

    // Validate the input against the schema
    wavelength.input.validate(&input)?;

    // Send the wavefront and return the link
    let link = send_wavefront(&prism_id, &frequency, input)?;

    // Process the response
    process_response(link, &wavelength.output, output)
}

fn send_wavefront(prism_id: &String, frequency: &str, input: Value) -> Result<UVLink> {
    let multiplexer = PrismMultiplexer::new();

    // Establish link to prism
    let link = multiplexer.establish_link(prism_id)?;

    // Send the wavefront
    link.send_wavefront(Uuid::new_v4(), prism_id, frequency, input)?;

    Ok(link)
}

fn process_photons(link: UVLink, output_schema: &UVSchemaDefinition, output: Option<&String>) -> Result<()> {
    loop {
        match link.receive() {
            Ok(Some((_id, pulse))) => {
                match pulse {
                    uv_core::UVPulse::Photon(photon) => {
                        render_photon(&photon.data, output_schema, output)?;
                    },
                    uv_core::UVPulse::Wavefront(wavefront) =>
                        return Err(anyhow::format_err!("Unexpected wavefront received {}", wavefront.id)),
                    uv_core::UVPulse::Trap(_trap) => break,
                    uv_core::UVPulse::Extinguish => break,
                }
            },
            Ok(None) => continue,
            Err(e) => return Err(anyhow::format_err!("Error receiving {}", e))
        }
    }

    Ok(())
}

fn render_photon(value: &Value, output_schema: &UVSchemaDefinition, output: Option<&String>) -> Result<()> {
    // Handle oneOf schemas with discriminated unions
    if output_schema.schema.get("oneOf").is_some() {
        return render_discriminated_union(value, output_schema, output);
    }

    // Handle simple schemas - use top-level type for routing
    match output_schema.schema.get("type") {
        Some(type_val) => match type_val.as_str() {
            Some("array") => render_array(value, output_schema, output),
            Some("object") => render_object(value, output_schema, output),
            Some(_) => render_object(value, output_schema, output), // Fallback
            None => render_object(value, output_schema, output), // Fallback
        },
        None => render_object(value, output_schema, output), // Fallback
    }
}

pub fn process_response(link: UVLink, output_schema: &UVSchemaDefinition, output: Option<&String>) -> Result<()> {
    // Special case: text streaming with x-uv-stream
    if let Some(stream_type) = output_schema.schema.get("x-uv-stream") {
        return process_stream(stream_type.to_string(), link, output_schema, output)
    }

    // General case: Process all photons until trap
    process_photons(link, output_schema, output)
}

fn render_discriminated_union(value: &Value, output_schema: &UVSchemaDefinition, output: Option<&String>) -> Result<()> {
    // Get the type discriminator from the data
    let event_type = value.get("type")
        .and_then(|t| t.as_str())
        .context("Missing 'type' field in discriminated union")?;

    // Find the matching schema in the oneOf array
    if let Some(one_of_array) = output_schema.schema.get("oneOf").and_then(|v| v.as_array()) {
        for schema_variant in one_of_array {
            // Check if this schema variant matches our event type
            if let Some(properties) = schema_variant.get("properties") {
                if let Some(type_constraint) = properties.get("type").and_then(|t| t.get("const")) {
                    if type_constraint.as_str() == Some(event_type) {
                        // Found matching schema - render based on its top-level type
                        return render_with_schema_variant(value, schema_variant, output);
                    }
                }
            }
        }
    }

    // Fallback if no matching schema found
    render_object(value, output_schema, output)
}

fn render_with_schema_variant(value: &Value, schema_variant: &Value, output: Option<&String>) -> Result<()> {
    // Use the top-level type from the matching schema variant for rendering
    match schema_variant.get("type").and_then(|t| t.as_str()) {
        Some("array") => {
            // Create temporary schema definition for the object variant
            let schema: UVSchemaDefinition = serde_json::from_value(schema_variant.clone())
                .context("Failed to parse schema variant")?;

            render_array(value, &schema, output)
        },
        Some("object") | _ => {
            // Create temporary schema definition for the object variant
            let schema: UVSchemaDefinition = serde_json::from_value(schema_variant.clone())
                .context("Failed to parse schema variant")?;
            
            render_object(value, &schema, output)
        }
    }
}

fn process_stream(stream_type: String, link: UVLink, output_schema: &UVSchemaDefinition, output: Option<&String>) -> Result<()> {
    loop {
        match link.receive() {
            Ok(Some((_id, pulse))) => {
                match pulse {
                    uv_core::UVPulse::Photon(photon) => {
                        render_stream(&stream_type, &photon.data, output_schema, output)?;
                    },
                    uv_core::UVPulse::Wavefront(wavefront) =>
                        return Err(anyhow::format_err!("Unexpected wavefront received {}", wavefront.id)),
                    uv_core::UVPulse::Trap(_trap) => break,
                    uv_core::UVPulse::Extinguish => break,
                }
            },
            Ok(None) => continue,
            Err(e) => return Err(anyhow::format_err!("Error receiving {}", e))
        }
    }

    Ok(())
}
