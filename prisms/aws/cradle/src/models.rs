use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct Timestamp {
    pub seconds: i64,
    pub nanos: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "createdDate")]
    pub created_date: Timestamp,
    #[serde(rename = "lastUpdatedDate")]
    pub last_updated_date: Timestamp,
    #[serde(rename = "lastUpdatedBy")]
    pub last_updated_by: String,
    pub version: i32,
    #[serde(rename = "profileState")]
    pub profile_state: String,
    #[serde(rename = "accountName")]
    pub account_name: String,
    pub name: String,
    pub description: String,
    pub closure: Value,  // Complex nested structure, use serde_json::Value
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CTI {
    pub category: String,
    #[serde(rename = "type")]
    pub cti_type: String,
    pub item: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobParameters {
    pub variables: Value,  // Dynamic key-value pairs
    pub schedule: Value,   // Complex schedule configuration
    pub dependencies: Vec<Value>,  // Array of dependency objects
    pub alerts: Vec<Value>,
    #[serde(rename = "serviceTier")]
    pub service_tier: String,
    #[serde(rename = "loaderRegistrationOption")]
    pub loader_registration_option: String,
    pub disabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "createdDate")]
    pub created_date: Timestamp,
    #[serde(rename = "lastUpdatedBy")]
    pub last_updated_by: String,
    #[serde(rename = "lastUpdatedDate")]
    pub last_updated_date: Timestamp,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub cti: CTI,
    #[serde(rename = "dedupeString")]
    pub dedupe_string: String,
    #[serde(rename = "ticketingEnabled")]
    pub ticketing_enabled: bool,
    #[serde(rename = "ticketSeverity")]
    pub ticket_severity: i32,
    #[serde(rename = "isTicketDaytimeSevTwo")]
    pub is_ticket_daytime_sev_two: bool,
    pub name: String,
    #[serde(rename = "jobParameters")]
    pub job_parameters: JobParameters,
}

// Output structs for display
#[derive(Debug, Serialize)]
pub struct ProfileOutput {
    pub id: String,
    pub name: String,
    pub description: String,
    pub profile_state: String,
    pub created_by: String,
    pub last_updated_date: String,  // Formatted as human-readable date
}

#[derive(Debug, Serialize)]
pub struct JobOutput {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub last_updated_date: String,  // Formatted as human-readable date
    pub status: String,  // Derived from job_parameters.disabled
}
