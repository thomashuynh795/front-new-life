use crate::app::error::AppError;
use crate::modules::scan_tokens::application::ports::{ScanTokenRepository, ScanTokenService};
use crate::modules::scan_tokens::domain::entities::{
    ScanToken, ScanTokenStatus, TokenBatch, TokenBatchStatus,
};
use crate::modules::tags::application::ports::{
    AuditEventRepository, CryptoService, ItemRepository, TagRepository,
};
use crate::modules::tags::domain::entities::{AuditEvent, Item, Tag, TagMode, TagStatus};
use chrono::{Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

const COUNTER_INITIAL: i64 = 0;

pub struct EnrollTagUseCase {
    tag_repo: Arc<dyn TagRepository>,
    item_repo: Arc<dyn ItemRepository>,
    audit_repo: Arc<dyn AuditEventRepository>,
    token_repo: Arc<dyn ScanTokenRepository>,
    token_service: Arc<dyn ScanTokenService>,
    default_scan_token_batch_size: u32,
    default_scan_token_ttl_seconds: i64,
}

pub struct EnrollTagRequest {
    pub tag_uid: String,
    pub product_code: String,
    pub size: Option<String>,
    pub color: Option<String>,
    pub mode: TagMode,
}

#[derive(Debug)]
pub struct EnrollTagResponse {
    pub tag_id: Uuid,
    pub item_id: Uuid,
    pub mode: TagMode,
    pub payload: TagWritePayload,
}

#[derive(Debug)]
pub enum TagWritePayload {
    DynamicCmac {
        key_version: i32,
        counter_initial: i64,
        verify_format: String,
    },
    OneTimeTokens {
        batch_id: Uuid,
        records: Vec<OneTimeTokenRecord>,
    },
}

#[derive(Debug, Clone)]
pub struct OneTimeTokenRecord {
    pub token_id: Uuid,
    pub token: String,
    pub url: String,
    pub expires_at: chrono::DateTime<Utc>,
}

impl EnrollTagUseCase {
    pub fn new(
        tag_repo: Arc<dyn TagRepository>,
        item_repo: Arc<dyn ItemRepository>,
        audit_repo: Arc<dyn AuditEventRepository>,
        token_repo: Arc<dyn ScanTokenRepository>,
        token_service: Arc<dyn ScanTokenService>,
        default_scan_token_batch_size: u32,
        default_scan_token_ttl_seconds: i64,
    ) -> Self {
        Self {
            tag_repo,
            item_repo,
            audit_repo,
            token_repo,
            token_service,
            default_scan_token_batch_size,
            default_scan_token_ttl_seconds,
        }
    }

    pub async fn execute(&self, request: EnrollTagRequest) -> Result<EnrollTagResponse, AppError> {
        let normalized_uid = normalize_tag_uid(&request.tag_uid)?;
        validate_product_code(&request.product_code)?;

        if self.tag_repo.find_by_uid(&normalized_uid).await?.is_some() {
            return Err(AppError::TagAlreadyExists);
        }

        let now = Utc::now();
        let tag_id = Uuid::new_v4();
        let item_id = Uuid::new_v4();

        let tag = Tag {
            id: tag_id,
            tag_uid: normalized_uid.clone(),
            mode: request.mode.clone(),
            status: TagStatus::Active,
            key_version: 1,
            last_counter: None,
            created_at: now,
            updated_at: now,
        };
        self.tag_repo.save(&tag).await?;

        let item = Item {
            id: item_id,
            product_code: request.product_code.clone(),
            size: request.size.clone(),
            color: request.color.clone(),
            tag_id,
            created_at: now,
            updated_at: now,
        };
        self.item_repo.save(&item).await?;

        let payload = match request.mode {
            TagMode::DynamicCmac => TagWritePayload::DynamicCmac {
                key_version: tag.key_version,
                counter_initial: COUNTER_INITIAL,
                verify_format:
                    r#"POST /verify {"tag_uid":"<tag_uid>","counter":<counter>,"cmac":"<hex>"}"#
                        .to_string(),
            },
            TagMode::OneTimeTokens => {
                let records = self
                    .generate_token_batch(
                        tag_id,
                        &request.product_code,
                        self.default_scan_token_batch_size,
                        self.default_scan_token_ttl_seconds,
                    )
                    .await?;
                TagWritePayload::OneTimeTokens {
                    batch_id: records.batch_id,
                    records: records.records,
                }
            }
        };

        self.audit_repo
            .save(&AuditEvent {
                id: Uuid::new_v4(),
                tag_id: Some(tag_id),
                event_type: "TAG_ENROLLED".to_string(),
                metadata: Some(
                    json!({
                        "mode": tag.mode.as_str(),
                        "product_code": request.product_code,
                    })
                    .to_string(),
                ),
                created_at: now,
            })
            .await?;

        Ok(EnrollTagResponse {
            tag_id,
            item_id,
            mode: tag.mode,
            payload,
        })
    }

    pub async fn generate_token_batch(
        &self,
        tag_id: Uuid,
        product_code: &str,
        count: u32,
        ttl_seconds: i64,
    ) -> Result<GeneratedBatch, AppError> {
        if count == 0 || count > 1_000 {
            return Err(AppError::Validation(
                "count must be between 1 and 1000".to_string(),
            ));
        }

        if ttl_seconds <= 0 {
            return Err(AppError::Validation(
                "ttl_seconds must be greater than 0".to_string(),
            ));
        }

        let now = Utc::now();
        let expires_at = now + Duration::seconds(ttl_seconds);
        let batch_id = Uuid::new_v4();

        self.token_repo
            .save_batch(&TokenBatch {
                id: batch_id,
                tag_id: Some(tag_id),
                product_public_id: product_code.to_string(),
                status: TokenBatchStatus::Active,
                expires_at,
                created_at: now,
                revoked_at: None,
            })
            .await?;

        let mut tokens = Vec::with_capacity(count as usize);
        let mut records = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let generated = self
                .token_service
                .generate_token(product_code, expires_at)?;
            tokens.push(ScanToken {
                token_id: generated.token_id,
                batch_id: Some(batch_id),
                tag_id: Some(tag_id),
                product_public_id: product_code.to_string(),
                expires_at,
                status: ScanTokenStatus::Unused,
                created_at: now,
                used_at: None,
                used_ip: None,
                used_user_agent: None,
                revoked_at: None,
                token_hash: self.token_service.hash_token(&generated.token),
            });
            records.push(OneTimeTokenRecord {
                token_id: generated.token_id,
                url: format!("/v1/scan?pid={product_code}&t={}", generated.token),
                token: generated.token,
                expires_at,
            });
        }

        self.token_repo.save_many(&tokens).await?;

        Ok(GeneratedBatch { batch_id, records })
    }
}

pub struct GeneratedBatch {
    pub batch_id: Uuid,
    pub records: Vec<OneTimeTokenRecord>,
}

pub fn normalize_tag_uid(value: &str) -> Result<String, AppError> {
    let normalized = value.trim().to_ascii_uppercase();
    if normalized.is_empty()
        || normalized.len() % 2 != 0
        || !normalized.chars().all(|char| char.is_ascii_hexdigit())
    {
        return Err(AppError::Validation(
            "tag_uid must be a non-empty even-length hex string".to_string(),
        ));
    }

    Ok(normalized)
}

pub fn validate_product_code(value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(
            "product_code cannot be empty".to_string(),
        ));
    }

    Ok(())
}

pub fn build_dynamic_message(tag_uid: &str, counter: i64) -> Result<Vec<u8>, AppError> {
    if counter < 0 {
        return Err(AppError::Validation(
            "counter must be greater than or equal to 0".to_string(),
        ));
    }

    let mut message = hex::decode(tag_uid).map_err(|_| {
        AppError::Validation("tag_uid must be a valid uppercase hex string".to_string())
    })?;
    message.extend_from_slice(&counter.to_be_bytes());
    Ok(message)
}

#[allow(dead_code)]
fn _assert_crypto_trait_object_safe(_crypto: Arc<dyn CryptoService>) {}
