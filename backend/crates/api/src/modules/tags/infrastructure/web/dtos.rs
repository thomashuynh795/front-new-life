use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Deserialize)]
pub struct EnrollTagRequest {
    pub tag_uid: String,
    pub product_code: String,
    pub size: Option<String>,
    pub color: Option<String>,
    pub mode: String,
}

#[derive(Serialize)]
pub struct EnrollTagResponse {
    pub tag_id: Uuid,
    pub item_id: Uuid,
    pub mode: String,
    pub payload: TagPayloadDto,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum TagPayloadDto {
    #[serde(rename = "dynamic_cmac")]
    DynamicCmac {
        key_version: i32,
        counter_initial: i64,
        verify_format: String,
    },
    #[serde(rename = "one_time_tokens")]
    OneTimeTokens {
        batch_id: Uuid,
        records: Vec<GeneratedTokenDto>,
    },
}

#[derive(Serialize, Clone)]
pub struct GeneratedTokenDto {
    pub token_id: Uuid,
    pub token: String,
    pub url: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct VerifyTagRequest {
    pub tag_uid: String,
    pub counter: i64,
    pub cmac: String,
}

#[derive(Serialize)]
pub struct VerifyTagResponse {
    pub verdict: String,
    pub product: Option<ProductDto>,
}

#[derive(Serialize)]
pub struct ProductDto {
    pub tag_id: Uuid,
}

#[derive(Serialize)]
pub struct RotateKeyResponse {
    pub new_key_version: i32,
    pub counter_reset: bool,
    pub message: String,
}

#[derive(Deserialize)]
pub struct ReconfigureTagRequest {
    pub reset_counter: Option<bool>,
    pub rotate_key: Option<bool>,
    pub revoke_existing_batch: Option<bool>,
    pub token_count: Option<u32>,
    pub ttl_seconds: Option<i64>,
}

#[derive(Serialize)]
pub struct ReconfigureTagResponse {
    pub tag_id: Uuid,
    pub mode: String,
    pub payload: ReconfigurePayloadDto,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ReconfigurePayloadDto {
    #[serde(rename = "dynamic_cmac")]
    DynamicCmac {
        key_version: i32,
        counter_initial: i64,
    },
    #[serde(rename = "one_time_tokens")]
    OneTimeTokens {
        revoked_batches: u64,
        batch_id: Uuid,
        records: Vec<GeneratedTokenDto>,
    },
}

#[derive(Deserialize)]
pub struct NextMessagesRequest {
    pub count: Option<u32>,
    pub starting_counter: Option<i64>,
}

#[derive(Serialize)]
pub struct NextMessagesResponse {
    pub tag_id: Uuid,
    pub tag_uid: String,
    pub key_version: i32,
    pub messages: Vec<GeneratedMessageDto>,
    pub verify_hint: VerifyHintDto,
}

#[derive(Serialize)]
pub struct GeneratedMessageDto {
    pub counter: i64,
    pub cmac: String,
}

#[derive(Serialize)]
pub struct VerifyHintDto {
    pub endpoint: String,
    pub body_template: serde_json::Value,
}

#[derive(Serialize)]
pub struct CatalogItemDto {
    pub item_id: Uuid,
    pub product_code: String,
    pub size: Option<String>,
    pub color: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tag: CatalogTagSummaryDto,
}

#[derive(Serialize)]
pub struct CatalogTagDto {
    pub tag_id: Uuid,
    pub tag_uid: String,
    pub mode: String,
    pub status: String,
    pub key_version: i32,
    pub last_counter: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub item: Option<CatalogItemSummaryDto>,
}

#[derive(Serialize)]
pub struct CatalogTagSummaryDto {
    pub tag_id: Uuid,
    pub tag_uid: String,
    pub mode: String,
    pub status: String,
    pub key_version: i32,
    pub last_counter: Option<i64>,
}

#[derive(Serialize)]
pub struct CatalogItemSummaryDto {
    pub item_id: Uuid,
    pub product_code: String,
    pub size: Option<String>,
    pub color: Option<String>,
}
