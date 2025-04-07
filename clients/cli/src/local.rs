use anyhow::{Context, Result};
use serde_json::Value;
use uuid::Uuid;
use uv_core::{PrismMultiplexer, UVLink, UVSchemaDefinition, UVSpectrum};

use crate::{parsing::cli_args::parse_args_to_map, rendering::cli_renderer::{render_array, render_object, render_stream}};


pub fn handle_local(prism: &str, args: Vec<String>) -> Result<()> {
    // Find the spectrum for the given prism
    let spectrum = UVSpectrum::new(&prism)
        .context(format!("Unable to find spectrum for prism {}", prism))?;

    // Extract the frequency from the arguments
    let (frequency, args) = args.split_first()
        .context("No frequency provided")?;

    // Find the wavelength in the spectrum for the given prism
    let wavelength = spectrum.find_wavelength(frequency)
        .context(format!("Unable to find frequency {} on prism {}", frequency, prism))?;

    // Parse the remaining arguments into an input object
    let input = serde_json::Value::Object(parse_args_to_map(args));

    // Validate the input against the schema
    wavelength.input.validate(&input)?;

    // Send the wavefront and return the link
    let link = send_wavefront(prism, &frequency, input)?;

    // Process the response
    process_response(link, &wavelength.output)
}

fn send_wavefront(prism: &str, frequency: &str, input: Value) -> Result<UVLink> {
    let multiplexer = PrismMultiplexer::new();

    // Establish link to prism
    let link = multiplexer.establish_link(prism)?;

    // Send the wavefront
    link.send_wavefront(Uuid::new_v4(), prism, frequency, input)?;

    Ok(link)
}

fn process_response(link: UVLink, output_schema: &UVSchemaDefinition) -> Result<()> {
    // see if we're dealing with a stream
    if let Some(stream_type) = output_schema.schema.get("x-uv-stream") {
        return process_stream(stream_type.to_string(), link, output_schema)
    }

    // If it's not a stream we can absorb the response
    let value = link.absorb::<Value>()?;

    match output_schema.schema
        .get("type")
        .unwrap()
        .as_str() {
            Some("array") => render_array(&value, output_schema),
            Some("object") => render_object(&value, output_schema),
            Some(_) => todo!(),
            None => todo!(),
        }
}

fn process_stream(stream_type: String, link: UVLink, output_schema: &UVSchemaDefinition) -> Result<()> {
    loop {
        match link.receive() {
            Ok(Some((_id, pulse))) => {
                match pulse {
                    uv_core::UVPulse::Photon(photon) => {
                        render_stream(&stream_type, &photon.data, output_schema)?;
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