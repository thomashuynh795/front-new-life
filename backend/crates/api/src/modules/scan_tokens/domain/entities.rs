use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanToken {
    pub token_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub tag_id: Option<Uuid>,
    pub product_public_id: String,
    pub expires_at: DateTime<Utc>,
    pub status: ScanTokenStatus,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub used_ip: Option<String>,
    pub used_user_agent: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub token_hash: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanTokenStatus {
    Unused,
    Used,
    Revoked,
}

impl ScanTokenStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanTokenStatus::Unused => "UNUSED",
            ScanTokenStatus::Used => "USED",
            ScanTokenStatus::Revoked => "REVOKED",
        }
    }
}

impl std::fmt::Display for ScanTokenStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenBatch {
    pub id: Uuid,
    pub tag_id: Option<Uuid>,
    pub product_public_id: String,
    pub status: TokenBatchStatus,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenBatchStatus {
    Active,
    Revoked,
}

impl TokenBatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenBatchStatus::Active => "ACTIVE",
            TokenBatchStatus::Revoked => "REVOKED",
        }
    }
}

impl std::fmt::Display for TokenBatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StaticScanResult {
    Ok,
    Replay,
    Invalid,
    Expired,
    Revoked,
    NotFound,
}

impl StaticScanResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            StaticScanResult::Ok => "OK",
            StaticScanResult::Replay => "REPLAY",
            StaticScanResult::Invalid => "INVALID",
            StaticScanResult::Expired => "EXPIRED",
            StaticScanResult::Revoked => "REVOKED",
            StaticScanResult::NotFound => "NOT_FOUND",
        }
    }
}

impl std::fmt::Display for StaticScanResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
