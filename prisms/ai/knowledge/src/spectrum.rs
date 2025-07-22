use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize)]
pub struct StoreRequest {
    pub content: String,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct RetrieveRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub id: String,
    pub content: Option<String>,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListTagsRequest {
    pub prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnowledgeEntry {
    pub id: String,
    pub title: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub relevance_score: f64,
}

fn default_limit() -> usize {
    10
}