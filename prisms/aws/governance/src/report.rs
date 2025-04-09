use crate::models::{TicketComment, TicketData, TicketStats, TicketSummary};
use std::collections::HashMap;
use std::error::Error;

pub fn generate_html_report(
    tickets: &[TicketData],
    summaries: &[TicketSummary],
    comments: &[TicketComment],
) -> Result<String, Box<dyn Error>> {
    let stats = TicketStats::new(tickets, summaries);
    
    // Create HTML report
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AWS Governance Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        h1, h2, h3 { color: #232f3e; }
        .card { background-color: #fff; border-radius: 4px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); margin-bottom: 20px; padding: 20px; }
        .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; }
        .stat-item { background: #f8f8f8; padding: 15px; border-radius: 4px; text-align: center; }
        .stat-value { font-size: 24px; font-weight: bold; color: #232f3e; }
        .stat-label { font-size: 14px; color: #666; }
        table { width: 100%; border-collapse: collapse; margin-top: 20px; }
        th, td { padding: 10px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background-color: #f2f2f2; }
        .status-open { color: #e77600; }
        .status-closed { color: #008a00; }
        .chart { height: 300px; }
    </style>
</head>
<body>
    <h1>AWS Governance Report</h1>"#);

    // Summary section
    html.push_str(r#"
    <div class="card">
        <h2>Summary</h2>
        <div class="stats">
            <div class="stat-item">
                <div class="stat-value">"#);
    html.push_str(&stats.total_tickets.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Total Tickets</div>
            </div>
            <div class="stat-item">
                <div class="stat-value">"#);
    html.push_str(&stats.open_tickets.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Open Tickets</div>
            </div>
            <div class="stat-item">
                <div class="stat-value">"#);
    html.push_str(&stats.closed_tickets.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Closed Tickets</div>
            </div>
            <div class="stat-item">
                <div class="stat-value">"#);
    html.push_str(&format!("{:.1}", stats.avg_days_open));
    html.push_str(r#"</div>
                <div class="stat-label">Avg Days Open</div>
            </div>
            <div class="stat-item">
                <div class="stat-value">"#);
    html.push_str(&stats.old_tickets.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Tickets > 30 Days</div>
            </div>
        </div>
    </div>"#);

    // Status distribution
    html.push_str(r#"
    <div class="card">
        <h2>Status Distribution</h2>
        <table>
            <tr>
                <th>Status</th>
                <th>Count</th>
            </tr>"#);

    // Sort status by count
    let mut status_items: Vec<(&String, &usize)> = stats.status_distribution.iter().collect();
    status_items.sort_by(|a, b| b.1.cmp(a.1));
    
    for (status, count) in status_items {
        html.push_str(&format!(r#"
            <tr>
                <td>{}</td>
                <td>{}</td>
            </tr>"#, status, count));
    }
    
    html.push_str(r#"
        </table>
    </div>"#);

    // Marketplace distribution
    html.push_str(r#"
    <div class="card">
        <h2>Marketplace Distribution</h2>
        <table>
            <tr>
                <th>Marketplace ID</th>
                <th>Count</th>
            </tr>"#);

    // Sort marketplace by count
    let mut marketplace_items: Vec<(&i32, &usize)> = stats.marketplace_distribution.iter().collect();
    marketplace_items.sort_by(|a, b| b.1.cmp(a.1));
    
    for (marketplace_id, count) in marketplace_items {
        html.push_str(&format!(r#"
            <tr>
                <td>{}</td>
                <td>{}</td>
            </tr>"#, marketplace_id, count));
    }
    
    html.push_str(r#"
        </table>
    </div>"#);

    // Tickets table
    html.push_str(r#"
    <div class="card">
        <h2>Tickets</h2>
        <table>
            <tr>
                <th>Ticket ID</th>
                <th>Status</th>
                <th>CTI</th>
                <th>Campaign</th>
                <th>Last Updated</th>
            </tr>"#);
    
    // Create a lookup map for summaries
    let summary_map: HashMap<_, _> = summaries.iter()
        .filter_map(|s| s.ticket_id.as_ref().map(|id| (id, s)))
        .collect();
    
    for ticket in tickets {
        let summary = summary_map.get(&ticket.ticket_id);
        
        // Create a long-lived string for the status
        let default_status = String::from("-");
        let status_str = summary
            .and_then(|s| s.status.as_ref())
            .unwrap_or(&default_status);
        
        let status_class = if status_str == "Closed" || status_str == "Resolved" { 
            "status-closed"
        } else { 
            "status-open" 
        };

        let status = status_str;
        
        let last_updated = summary
            .and_then(|s| s.last_updated_at.as_ref())
            .map(|d| d.split('T').next().unwrap_or("-"))
            .unwrap_or("-");

        html.push_str(&format!(r#"
            <tr>
                <td><a href="https://t.corp.amazon.com/{}" target="_blank">{}</a></td>
                <td class="{}">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>"#, 
            ticket.ticket_id, ticket.ticket_id,
            status_class, status,
            ticket.cti,
            ticket.campaign,
            last_updated
        ));
    }
    
    html.push_str(r#"
        </table>
    </div>"#);
    
    // Comments section
    if !comments.is_empty() {
        html.push_str(r#"
    <div class="card">
        <h2>Recent Comments</h2>
        <table>
            <tr>
                <th>Ticket ID</th>
                <th>Date</th>
                <th>Comment</th>
            </tr>"#);
        
        // Group comments by ticket_id
        let mut comments_by_ticket: HashMap<String, Vec<&TicketComment>> = HashMap::new();
        
        for comment in comments {
            if let Some(ticket_id) = &comment.ticket_id {
                comments_by_ticket
                    .entry(ticket_id.clone())
                    .or_insert_with(Vec::new)
                    .push(comment);
            }
        }
        
        // Sort each group by date and take the most recent
        for (ticket_id, ticket_comments) in &mut comments_by_ticket {
            ticket_comments.sort_by(|a, b| {
                b.created_at
                    .as_ref()
                    .unwrap_or(&String::from(""))
                    .cmp(a.created_at.as_ref().unwrap_or(&String::from("")))
            });
            
            if let Some(comment) = ticket_comments.first() {
                let default_date = "-";
                let date = comment
                    .created_at
                    .as_ref()
                    .and_then(|d| d.split('T').next())
                    .unwrap_or(default_date);
                
                let default_message = String::from("-");
                let message = comment
                    .message
                    .as_ref()
                    .unwrap_or(&default_message);
                
                html.push_str(&format!(r#"
                <tr>
                    <td><a href="https://t.corp.amazon.com/{}" target="_blank">{}</a></td>
                    <td>{}</td>
                    <td>{}</td>
                </tr>"#, 
                    ticket_id, ticket_id,
                    date,
                    message
                ));
            }
        }
        
        html.push_str(r#"
        </table>
    </div>"#);
    }
    
    // Close document
    html.push_str(r#"
</body>
</html>"#);

    Ok(html)
}
