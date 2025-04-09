//! Typed structures for the governance prism spectrum.
//!
//! This module defines the input and output structures for the governance prism,
//! providing type safety and better developer experience.

use serde::{Deserialize, Serialize};

// Input/output types for tickets.list
#[derive(Debug, Clone, Deserialize)]
pub struct TicketsListInput {}

#[derive(Debug, Clone, Serialize)]
pub struct TicketsListOutput {
    pub tickets: Vec<TicketOutput>,
}

// Input/output types for performers.list
#[derive(Debug, Clone, Deserialize)]
pub struct PerformersListInput {}

#[derive(Debug, Clone, Serialize)]
pub struct PerformersListOutput {
    pub performers: Vec<PerformerOutput>,
}

// Input/output types for report
#[derive(Debug, Clone, Deserialize)]
pub struct ReportInput {}

#[derive(Debug, Clone, Serialize)]
pub struct ReportOutput {
    pub report: String,
}

// Common output structures
#[derive(Debug, Clone, Serialize)]
pub struct TicketOutput {
    pub cti: String,
    #[serde(rename = "widgetGroup")]
    pub widget_group: String,
    pub campaign: String,
    #[serde(rename = "marketplaceId")]
    pub marketplace_id: String,
    #[serde(rename = "lowCtrFlag")]
    pub low_ctr_flag: String,
    #[serde(rename = "ticketId")]
    pub ticket_id: String,
    pub status: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    #[serde(rename = "displayTicket")]
    pub display_ticket: String,
    #[serde(rename = "ticketUrl")]
    pub ticket_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformerOutput {
    pub cti: String,
    #[serde(rename = "widgetGroup")]
    pub widget_group: String,
    pub campaign: String,
    #[serde(rename = "marketplaceId")]
    pub marketplace_id: String,
    #[serde(rename = "lowCtrFlag")]
    pub low_ctr_flag: String,
    #[serde(rename = "serverCtr")]
    pub server_ctr: String,
}

// HTTP response type for curl refraction
#[derive(Debug, Clone, Deserialize)]
pub struct HttpResponse {
    pub status: i32,
    pub body: String,
}
