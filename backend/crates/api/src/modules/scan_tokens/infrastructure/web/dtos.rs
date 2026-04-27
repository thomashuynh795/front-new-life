use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct GenerateScanTokensRequest {
    pub count: u32,
    pub ttl_seconds: i64,
}

#[derive(Serialize)]
pub struct GenerateScanTokensResponse {
    pub product_public_id: String,
    pub batch_id: Uuid,
    pub tokens: Vec<GeneratedScanTokenDto>,
}

#[derive(Serialize)]
pub struct GeneratedScanTokenDto {
    pub token_id: Uuid,
    pub token: String,
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ScanQuery {
    pub pid: String,
    pub t: String,
}

#[derive(Serialize)]
pub struct ScanTokenResponse {
    pub product_public_id: String,
    pub result: String,
    pub authentic: bool,
}
