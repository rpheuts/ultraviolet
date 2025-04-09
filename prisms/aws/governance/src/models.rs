use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct TicketComment {
    #[serde(rename = "ticketId")]
    pub ticket_id: Option<String>,
    pub message: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}

pub struct TicketStats {
    pub total_tickets: usize,
    pub open_tickets: usize,
    pub closed_tickets: usize,
    pub avg_days_open: f64,
    pub old_tickets: usize,
    pub status_distribution: HashMap<String, usize>,
    pub marketplace_distribution: HashMap<i32, usize>,
}

impl TicketStats {
    pub fn new(tickets: &[TicketData], summaries: &[TicketSummary]) -> Self {
        let mut stats = TicketStats {
            total_tickets: tickets.len(),
            open_tickets: 0,
            closed_tickets: 0,
            avg_days_open: 0.0,
            old_tickets: 0,
            status_distribution: HashMap::new(),
            marketplace_distribution: HashMap::new(),
        };

        let now = chrono::Utc::now();
        let mut total_days = 0.0;
        let mut ticket_count = 0;

        for ticket in tickets {
            // Track marketplace distribution
            *stats.marketplace_distribution.entry(ticket.marketplace_id).or_insert(0) += 1;

            if let Some(summary) = summaries.iter().find(|s| s.ticket_id.as_ref() == Some(&ticket.ticket_id)) {
                // Update status counts
                if let Some(status) = &summary.status {
                    *stats.status_distribution.entry(status.clone()).or_insert(0) += 1;
                    
                    if status == "Closed" || status == "Resolved" {
                        stats.closed_tickets += 1;
                    } else {
                        stats.open_tickets += 1;
                    }
                }

                // Calculate age
                if let Some(created_at) = &summary.created_at {
                    if let Ok(created) = chrono::DateTime::parse_from_rfc3339(created_at) {
                        let age = (now - created.with_timezone(&chrono::Utc)).num_days();
                        total_days += age as f64;
                        ticket_count += 1;

                        if age > 30 {
                            stats.old_tickets += 1;
                        }
                    }
                }
            }
        }

        if ticket_count > 0 {
            stats.avg_days_open = total_days / ticket_count as f64;
        }

        stats
    }
}

#[derive(Debug, Deserialize)]
pub struct TicketSummary {
    #[serde(rename = "ticketId")]
    pub ticket_id: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "lastUpdatedAt")]
    pub last_updated_at: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TicketData {
    #[serde(rename = "symphony_cti")]
    pub cti: String,
    #[serde(rename = "widget_group_id")]
    pub widget_group: String,
    #[serde(rename = "symphony_name")]
    pub campaign: String,
    #[serde(rename = "marketplace_id")]
    pub marketplace_id: i32,
    #[serde(rename = "low_client_ctr_flag")]
    pub low_ctr_flag: String,
    #[serde(rename = "ticket_id")]
    pub ticket_id: String,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize)]
pub struct PerformerData {
    #[serde(rename = "symphony_cti")]
    pub cti: String,
    #[serde(rename = "widget_group_id")]
    pub widget_group: String,
    #[serde(rename = "symphony_name")]
    pub campaign: String,
    #[serde(rename = "marketplace_id")]
    pub marketplace_id: i32,
    #[serde(rename = "low_client_ctr_flag")]
    pub low_ctr_flag: String,
    #[serde(rename = "t6w_server_ctr")]
    pub server_ctr: Option<f64>,
}

#[derive(Debug, Serialize)]
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
