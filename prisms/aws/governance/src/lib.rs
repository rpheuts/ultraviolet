//! Governance prism implementation for the Ultraviolet system.
//!
//! This prism provides capabilities for managing AWS governance tickets and performers
//! through internal Amazon APIs.

pub mod spectrum;
mod models;
mod report;

use std::io::Read;
use models::{PerformerData, TicketComment, TicketData, TicketSummary};
use serde_json::{json, Value};
use spectrum::{HttpResponse, PerformerOutput, TicketOutput, TicketsListOutput, ReportOutput};
use uv_core::{UVError, UVLink, UVPrism, UVPulse, UVSpectrum, PrismMultiplexer, Result};
use uuid::Uuid;
use hex;
use flate2::read::ZlibDecoder;

/// Governance prism for managing tickets and performers
pub struct GovernancePrism {
    spectrum: Option<UVSpectrum>,
    multiplexer: PrismMultiplexer,
}

impl GovernancePrism {
    /// Create a new governance prism.
    pub fn new() -> Self {
        Self {
            spectrum: None,
            multiplexer: PrismMultiplexer::new(),
        }
    }

    fn format_ctr(ctr: Option<f64>) -> String {
        ctr.map(|v| format!("{:.2}%", v * 100.0))
            .unwrap_or_else(|| "-".to_string())
    }

    fn get_tickets(&self) -> Result<Vec<TicketData>> {
        // Use the curl.post refraction
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://api.ce.kindle.amazon.dev/governance?org=hottier&action=GetPersistedPerformers&marketplace=1",
                "headers": {
                    "referer": "https://console.harmony.a2z.com/"
                },
                "body": ""
            }),
        )?;

        // Extract body from response
        let body = &response.body;

        // Parse tickets from response
        serde_json::from_str(body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse response: {}", e)))
    }

    fn get_ticket_summaries(&self, tickets: &[TicketData]) -> Result<Vec<TicketSummary>> {
        // Extract ticket IDs
        let ticket_ids: Vec<String> = tickets
            .iter()
            .filter(|t| !t.ticket_id.is_empty())
            .map(|t| t.ticket_id.clone())
            .collect();

        // If no tickets, return empty vec
        if ticket_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Request summaries
        let body_json = serde_json::to_string(&ticket_ids)
            .map_err(|e| UVError::ExecutionError(format!("Failed to serialize ticket IDs: {}", e)))?;

        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://api.ce.kindle.amazon.dev/governance?org=hottier&action=GetTicketsSummaries&marketplace=1",
                "headers": {
                    "referer": "https://console.harmony.a2z.com/",
                    "content-type": "application/json"
                },
                "body": body_json
            }),
        )?;

        // Parse summaries from response
        serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse summaries: {}", e)))
    }

    fn get_ticket_comments(&self, tickets: &[TicketData]) -> Result<Vec<TicketComment>> {
        let mut all_comments = Vec::new();

        for ticket in tickets {
            if ticket.ticket_id.is_empty() {
                continue;
            }

            let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
                "curl.post",
                self.spectrum.as_ref().unwrap(),
                json!({
                    "url": "https://api.ce.kindle.amazon.dev/governance?org=hottier&action=GetTicketComments&marketplace=1",
                    "headers": {
                        "referer": "https://console.harmony.a2z.com/",
                        "content-type": "text/plain"
                    },
                    "body": ticket.ticket_id
                }),
            )?;

            if let Ok(mut comments) = serde_json::from_str::<Vec<TicketComment>>(&response.body) {
                all_comments.append(&mut comments);
            }

            // Wait 1 second between requests to respect rate limiting
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        Ok(all_comments)
    }

    /// Handle tickets.list frequency
    fn handle_tickets_list(&self, id: Uuid, _input: Value, link: &UVLink) -> Result<()> {
        let tickets = self.get_tickets()?;
        if tickets.is_empty() {
            let output = TicketsListOutput { tickets: vec![] };
            link.emit_photon(id, serde_json::to_value(output)?)?;
            link.emit_trap(id, None)?;
            return Ok(());
        }

        let summaries = self.get_ticket_summaries(&tickets)?;

        // Format output
        let output: Vec<TicketOutput> = tickets
            .into_iter()
            .map(|ticket| {
                let summary = summaries
                    .iter()
                    .find(|s| s.ticket_id.as_ref() == Some(&ticket.ticket_id));

                let last_updated = summary
                    .and_then(|s| s.last_updated_at.as_ref())
                    .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "-".to_string());

                TicketOutput {
                    cti: ticket.cti,
                    widget_group: ticket.widget_group,
                    campaign: ticket.campaign,
                    marketplace_id: ticket.marketplace_id.to_string(),
                    low_ctr_flag: ticket.low_ctr_flag,
                    ticket_id: ticket.ticket_id.clone(),
                    status: summary
                        .and_then(|s| s.status.clone())
                        .unwrap_or_else(|| "-".to_string()),
                    last_updated,
                    display_ticket: format!("🔗 {}", ticket.ticket_id),
                    ticket_url: format!("https://t.corp.amazon.com/{}", ticket.ticket_id),
                }
            })
            .collect();

        for ticket in output {
            link.emit_photon(id, serde_json::to_value(ticket)?)?;
        }

        link.emit_trap(id, None)?;

        Ok(())
    }

    /// Handle performers.list frequency
    fn handle_performers_list(&self, id: Uuid, _input: Value, link: &UVLink) -> Result<()> {
        let response = self.multiplexer.refract_and_absorb::<HttpResponse>(
            "curl.post",
            self.spectrum.as_ref().unwrap(),
            json!({
                "url": "https://api.ce.kindle.amazon.dev/governance?org=hottier&action=GetCurrentPerformers&marketplace=1",
                "headers": {
                    "referer": "https://console.harmony.a2z.com/"
                },
                "body": ""
            }),
        )?;

        // Check if the response is a hex-encoded compressed string
        // This is a simple heuristic - we check if it's a valid hex string
        if response.body.chars().all(|c| c.is_ascii_hexdigit()) {
            println!("Response is hex-encoded compressed data");
            
            // Decode hex string to bytes
            let bytes = hex::decode(&response.body)
                .map_err(|e| UVError::ExecutionError(format!("Failed to decode hex: {}", e)))?;
            
            println!("Decoded hex to {} bytes", bytes.len());
            
            // Decompress bytes
            let mut decoder = ZlibDecoder::new(&bytes[..]);
            let mut decompressed = String::new();
            decoder.read_to_string(&mut decompressed)
                .map_err(|e| UVError::ExecutionError(format!("Failed to decompress: {}", e)))?;
            
            println!("Decompressed length: {} bytes", decompressed.len());
            
            // Parse JSON
            let performers: Vec<PerformerData> = serde_json::from_str(&decompressed)
                .map_err(|e| UVError::ExecutionError(format!("Failed to parse decompressed response: {}", e)))?;

            let output: Vec<PerformerOutput> = performers
                .into_iter()
                .map(|performer| PerformerOutput {
                    cti: performer.cti,
                    widget_group: performer.widget_group,
                    campaign: performer.campaign,
                    marketplace_id: performer.marketplace_id.to_string(),
                    low_ctr_flag: performer.low_ctr_flag,
                    server_ctr: Self::format_ctr(performer.server_ctr),
                })
                .collect();

            for performer in output {
                link.emit_photon(id, serde_json::to_value(performer)?)?;
            }
            
            link.emit_trap(id, None)?;
            
            return Ok(());
        }
        
        // If not compressed, try parsing as regular JSON
        println!("Trying to parse as regular JSON");
        
        let performers: Vec<PerformerData> = serde_json::from_str(&response.body)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse response: {}", e)))?;

        let output: Vec<PerformerOutput> = performers
            .into_iter()
            .map(|performer| PerformerOutput {
                cti: performer.cti,
                widget_group: performer.widget_group,
                campaign: performer.campaign,
                marketplace_id: performer.marketplace_id.to_string(),
                low_ctr_flag: performer.low_ctr_flag,
                server_ctr: Self::format_ctr(performer.server_ctr),
            })
            .collect();

        for performer in output {
            link.emit_photon(id, serde_json::to_value(performer)?)?;
        }
        
        link.emit_trap(id, None)?;

        Ok(())
    }

    /// Handle report frequency
    fn handle_report(&self, id: Uuid, _input: Value, link: &UVLink) -> Result<()> {
        // Get tickets and summaries
        let tickets = self.get_tickets()?;
        let summaries = self.get_ticket_summaries(&tickets)?;
        
        // Get comments
        let comments = self.get_ticket_comments(&tickets)?;
        
        // Generate report
        let html = report::generate_html_report(&tickets, &summaries, &comments)
            .map_err(|e| UVError::ExecutionError(format!("Failed to generate report: {}", e)))?;
        
        // Return HTML in the response
        let output = ReportOutput { report: html };
        link.emit_photon(id, serde_json::to_value(output)?)?;
        link.emit_trap(id, None)?;
        
        Ok(())
    }
}

impl UVPrism for GovernancePrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "tickets.list" => {
                    self.handle_tickets_list(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "performers.list" => {
                    self.handle_performers_list(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                "report" => {
                    self.handle_report(id, wavefront.input.clone(), link)?;
                    return Ok(true);
                },
                _ => {
                    // Unknown frequency
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
}

// Export a function to create a new instance
// This will be used by the dynamic loading system
#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(GovernancePrism::new())
}
