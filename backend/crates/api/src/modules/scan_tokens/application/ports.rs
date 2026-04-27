use crate::app::error::AppError;
use crate::modules::scan_tokens::domain::entities::{ScanToken, TokenBatch};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait ScanTokenRepository: Send + Sync {
    async fn save_many(&self, tokens: &[ScanToken]) -> Result<(), AppError>;
    async fn save_batch(&self, batch: &TokenBatch) -> Result<TokenBatch, AppError>;
    async fn find_by_id(&self, token_id: Uuid) -> Result<Option<ScanToken>, AppError>;
    async fn revoke(&self, token_id: Uuid, revoked_at: DateTime<Utc>) -> Result<bool, AppError>;
    async fn revoke_active_batch_for_tag(
        &self,
        tag_id: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<u64, AppError>;
    async fn consume_if_unused(
        &self,
        token_id: Uuid,
        used_at: DateTime<Utc>,
        used_ip: Option<String>,
        used_user_agent: Option<String>,
    ) -> Result<bool, AppError>;
}

pub trait ScanTokenService: Send + Sync {
    fn generate_token(
        &self,
        product_public_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<GeneratedScanToken, AppError>;

    fn parse_and_verify_token(
        &self,
        product_public_id: &str,
        token: &str,
    ) -> Result<VerifiedScanToken, AppError>;

    fn hash_token(&self, token: &str) -> Vec<u8>;
}

#[derive(Debug)]
pub struct GeneratedScanToken {
    pub token_id: Uuid,
    pub token: String,
}

#[derive(Debug)]
pub struct VerifiedScanToken {
    pub token_id: Uuid,
    pub expires_at: DateTime<Utc>,
}
