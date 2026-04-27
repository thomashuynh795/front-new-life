use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: Uuid,
    pub tag_uid: String,
    pub mode: TagMode,
    pub status: TagStatus,
    pub key_version: i32,
    pub last_counter: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagMode {
    DynamicCmac,
    OneTimeTokens,
}

impl TagMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            TagMode::DynamicCmac => "dynamic_cmac",
            TagMode::OneTimeTokens => "one_time_tokens",
        }
    }
}

impl std::fmt::Display for TagMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for TagMode {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "dynamic_cmac" => Ok(Self::DynamicCmac),
            "one_time_tokens" => Ok(Self::OneTimeTokens),
            _ => Err(format!("Unsupported tag mode: {value}")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagStatus {
    Active,
    Revoked,
}

impl Default for TagStatus {
    fn default() -> Self {
        Self::Active
    }
}

impl TagStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TagStatus::Active => "ACTIVE",
            TagStatus::Revoked => "REVOKED",
        }
    }
}

impl std::fmt::Display for TagStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: Uuid,
    pub product_code: String,
    pub size: Option<String>,
    pub color: Option<String>,
    pub tag_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanEvent {
    pub id: Uuid,
    pub tag_id: Option<Uuid>,
    pub token_id: Option<Uuid>,
    pub tag_uid: String,
    pub product_public_id: Option<String>,
    pub received_counter: Option<i64>,
    pub verdict: String,
    pub metadata: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanVerdict {
    Valid,
    InvalidSignature,
    ReplayDetected,
    TagRevoked,
    TagNotFound,
}

impl ScanVerdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanVerdict::Valid => "VALID",
            ScanVerdict::InvalidSignature => "INVALID_SIGNATURE",
            ScanVerdict::ReplayDetected => "REPLAY_DETECTED",
            ScanVerdict::TagRevoked => "TAG_REVOKED",
            ScanVerdict::TagNotFound => "TAG_NOT_FOUND",
        }
    }
}

impl std::fmt::Display for ScanVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEvent {
    pub id: Uuid,
    pub tag_id: Option<Uuid>,
    pub event_type: String,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}
