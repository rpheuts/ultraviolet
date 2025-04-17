use serde::Deserialize;

// Input types for each frequency

#[derive(Debug, Deserialize)]
pub struct ProfilesListInput {
    pub account_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ProfilesGetInput {
    pub profile_id: String,
}

#[derive(Debug, Deserialize)]
pub struct JobsListInput {
    pub profile_id: String,
}

#[derive(Debug, Deserialize)]
pub struct JobsGetInput {
    pub profile_id: String,
    pub job_id: String,
}

// Response types for HTTP requests

#[derive(Debug, Deserialize)]
pub struct HttpResponse {
    pub status: i32,
    pub body: String
}
