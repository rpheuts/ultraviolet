use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct FavoriteAccount {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "accountName")]
    pub account_name: String,
    pub policy: String,
    #[serde(rename = "sessionDuration")]
    pub session_duration: String,
    pub partition: String,
}

// Refraction types
#[derive(Debug, Deserialize)]
pub struct FavoritesResponse {
    #[serde(rename = "favoriteAccountList")]
    pub favorite_account_list: Vec<FavoriteAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: i32,
    pub body: String,
}