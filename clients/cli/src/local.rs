use std::io::stdout;

use anyhow::{Context, Result};
use serde_json::Value;
use uuid::Uuid;
use uv_core::{PrismMultiplexer, UVSpectrum};
use uv_ui::UIInferenceEngine;

use crate::parsing::cli_args::parse_args_to_map;
use crate::rendering::cli_renderer::CliRenderer;


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

    // Send the wavefront and absorb the response
    let response = send_wavefront(prism, &frequency, input)?;

    // Use UI inference engine to determine the response layout
    let component = UIInferenceEngine::new().infer(&response)?;

    CliRenderer::new().render(&component, &mut stdout())?;

    Ok(())
}

fn send_wavefront(prism: &str, frequency: &str, input: Value) -> Result<Value> {
    let multiplexer = PrismMultiplexer::new();

    // Establish link to prism
    let link = multiplexer.establish_link(prism)?;

    // Send the wavefront
    link.send_wavefront(Uuid::new_v4(), prism, frequency, input)?;

    // Absorb the response
    Ok(link.absorb::<Value>()?)
}