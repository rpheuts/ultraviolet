//! Ray execution for the CLI

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;
use uv_core::{PrismMultiplexer, RayDefinition, Transmitter, UVLink};

use crate::rendering::cli_renderer::{render_object, render_stream};

/// Handle ray execution from CLI
pub fn handle_ray(ray_name: &str, args: Vec<String>, output: Option<&String>) -> Result<()> {
    // Load ray definition from file
    let ray_definition = load_ray_definition(ray_name)?;
    
    // Parse arguments into input JSON
    let input = parse_ray_input(&args)?;
    
    // Execute the ray
    execute_ray(&ray_definition, input, output)
}

/// Load ray definition from rays directory
fn load_ray_definition(ray_name: &str) -> Result<RayDefinition> {
    // Try different possible paths
    let possible_paths = vec![
        format!("rays/{}.json", ray_name),
        format!("rays/{}/{}.json", ray_name.split(':').next().unwrap_or("flows"), ray_name),
        format!("{}.json", ray_name),
    ];
    
    for path_str in possible_paths {
        let path = Path::new(&path_str);
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .context(format!("Failed to read ray definition from {}", path_str))?;
            
            let ray_definition: RayDefinition = serde_json::from_str(&content)
                .context(format!("Failed to parse ray definition from {}", path_str))?;
            
            return Ok(ray_definition);
        }
    }
    
    Err(anyhow::format_err!("Ray definition not found: {}", ray_name))
}

/// Parse CLI arguments into ray input JSON
fn parse_ray_input(args: &[String]) -> Result<Value> {
    if args.is_empty() {
        return Ok(serde_json::json!({}));
    }
    
    // If single argument that looks like JSON, parse it directly
    if args.len() == 1 {
        if let Ok(parsed) = serde_json::from_str::<Value>(&args[0]) {
            return Ok(parsed);
        }
    }
    
    // Otherwise, parse as key=value pairs
    let mut input = serde_json::Map::new();
    
    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            // Try to parse value as JSON, fallback to string
            let parsed_value = serde_json::from_str::<Value>(value)
                .unwrap_or_else(|_| Value::String(value.to_string()));
            input.insert(key.to_string(), parsed_value);
        } else {
            // Treat as boolean flag
            input.insert(arg.clone(), Value::Bool(true));
        }
    }
    
    Ok(Value::Object(input))
}

/// Execute a ray definition
fn execute_ray(ray_definition: &RayDefinition, input: Value, output: Option<&String>) -> Result<()> {
    // Create multiplexer and transmitter
    println!("Creating multiplexer and transmitter...");
    let multiplexer = std::sync::Arc::new(PrismMultiplexer::new());
    let transmitter = Transmitter::new(multiplexer);
    println!("Multiplexer and transmitter created");

    // Create output link for results
    let (output_link, result_link) = UVLink::create_link();
    let request_id = Uuid::new_v4();
    
    // Use a channel to coordinate between async execution and sync result processing
    let (tx, rx) = std::sync::mpsc::channel();
    
    // Start ray execution in a separate thread to avoid runtime conflicts
    let ray_def = ray_definition.clone();
    let input_clone = input.clone();
    let output_link_clone = output_link.clone();
    
    let execution_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            println!("Starting ray execution...");
            if let Err(e) = transmitter.execute_ray(&ray_def, input_clone, &output_link_clone, request_id).await {
                eprintln!("Ray execution failed: {}", e);
                let _ = output_link_clone.emit_trap(request_id, Some(e));
            } else {
                println!("Ray execution completed successfully");
            }
            let _ = tx.send(()); // Signal completion
        });
    });
    
    // Give execution thread more time to start and establish connections
    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    // Process results on main thread
    println!("Starting to process results...");
    let result = process_ray_results(result_link, &ray_definition.output, output);
    println!("Finished processing results");
    
    // Wait for execution to complete
    let _ = execution_handle.join();
    
    result
}

/// Process results from ray execution
fn process_ray_results(link: UVLink, output_schema: &Value, output: Option<&String>) -> Result<()> {
    // Check if this is a streaming output
    if let Some(stream_type) = output_schema.get("x-uv-stream") {
        return process_ray_stream(stream_type.to_string(), link, output_schema, output);
    }
    
    // Process regular photons
    println!("Starting to receive pulses...");
    loop {
        match link.receive() {
            Ok(Some((_id, pulse))) => {
                println!("Received pulse: {:?}", pulse);
                match pulse {
                    uv_core::UVPulse::Photon(photon) => {
                        render_ray_photon(&photon.data, output_schema, output)?;
                    },
                    uv_core::UVPulse::Trap(trap) => {
                        println!("Received trap: {:?}", trap);
                        if let Some(error) = trap.error {
                            return Err(anyhow::format_err!("Ray execution failed: {}", error));
                        }
                        break;
                    },
                    uv_core::UVPulse::Extinguish => {
                        println!("Received extinguish");
                        break;
                    },
                    _ => continue,
                }
            },
            Ok(None) => {
                println!("No pulse received, continuing...");
                std::thread::sleep(std::time::Duration::from_millis(50));
                continue;
            },
            Err(e) => {
                println!("Error receiving pulse: {}", e);
                return Err(anyhow::format_err!("Error receiving results: {}", e));
            }
        }
    }
    
    Ok(())
}

/// Process streaming results from ray execution
fn process_ray_stream(stream_type: String, link: UVLink, output_schema: &Value, output: Option<&String>) -> Result<()> {
    loop {
        match link.receive() {
            Ok(Some((_id, pulse))) => {
                match pulse {
                    uv_core::UVPulse::Photon(photon) => {
                        // Parse schema definition from JSON
                        let temp_schema: uv_core::UVSchemaDefinition = serde_json::from_value(output_schema.clone())
                            .context("Failed to parse output schema")?;
                        render_stream(&stream_type, &photon.data, &temp_schema, output)?;
                    },
                    uv_core::UVPulse::Trap(trap) => {
                        if let Some(error) = trap.error {
                            return Err(anyhow::format_err!("Ray execution failed: {}", error));
                        }
                        break;
                    },
                    uv_core::UVPulse::Extinguish => break,
                    _ => continue,
                }
            },
            Ok(None) => continue,
            Err(e) => return Err(anyhow::format_err!("Error receiving stream: {}", e)),
        }
    }
    
    Ok(())
}

/// Render a photon from ray execution
fn render_ray_photon(value: &Value, output_schema: &Value, output: Option<&String>) -> Result<()> {
    // Parse schema definition from JSON
    let temp_schema: uv_core::UVSchemaDefinition = serde_json::from_value(output_schema.clone())
        .context("Failed to parse output schema")?;
    
    // Use existing object renderer
    render_object(value, &temp_schema, output)
}