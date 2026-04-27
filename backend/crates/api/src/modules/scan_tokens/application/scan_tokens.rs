use crate::app::error::AppError;
use crate::modules::scan_tokens::application::ports::{ScanTokenRepository, ScanTokenService};
use crate::modules::scan_tokens::domain::entities::{
    ScanToken, ScanTokenStatus, StaticScanResult, TokenBatch, TokenBatchStatus,
};
use crate::modules::tags::application::ports::{ItemRepository, ScanEventRepository};
use crate::modules::tags::domain::entities::ScanEvent;
use chrono::{Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub struct GenerateScanTokensUseCase {
    item_repo: Arc<dyn ItemRepository>,
    token_repo: Arc<dyn ScanTokenRepository>,
    token_service: Arc<dyn ScanTokenService>,
}

pub struct GenerateScanTokensRequest {
    pub product_public_id: String,
    pub count: u32,
    pub ttl_seconds: i64,
}

pub struct GeneratedScanTokenUrl {
    pub token_id: Uuid,
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

pub struct GenerateScanTokensResponse {
    pub product_public_id: String,
    pub batch_id: Uuid,
    pub tokens: Vec<GeneratedScanTokenUrl>,
}

impl GenerateScanTokensUseCase {
    pub fn new(
        item_repo: Arc<dyn ItemRepository>,
        token_repo: Arc<dyn ScanTokenRepository>,
        token_service: Arc<dyn ScanTokenService>,
    ) -> Self {
        Self {
            item_repo,
            token_repo,
            token_service,
        }
    }

    pub async fn execute(
        &self,
        request: GenerateScanTokensRequest,
    ) -> Result<GenerateScanTokensResponse, AppError> {
        validate_generate_request(&request)?;

        let product_exists = self
            .item_repo
            .exists_by_product_code(&request.product_public_id)
            .await?;
        if !product_exists {
            return Err(AppError::ProductNotFound);
        }

        let now = Utc::now();
        let expires_at = now + Duration::seconds(request.ttl_seconds);
        let batch_id = Uuid::new_v4();

        self.token_repo
            .save_batch(&TokenBatch {
                id: batch_id,
                tag_id: None,
                product_public_id: request.product_public_id.clone(),
                status: TokenBatchStatus::Active,
                expires_at,
                created_at: now,
                revoked_at: None,
            })
            .await?;

        let mut tokens_to_store = Vec::with_capacity(request.count as usize);
        let mut generated_tokens = Vec::with_capacity(request.count as usize);

        for _ in 0..request.count {
            let generated = self
                .token_service
                .generate_token(&request.product_public_id, expires_at)?;
            tokens_to_store.push(ScanToken {
                token_id: generated.token_id,
                batch_id: Some(batch_id),
                tag_id: None,
                product_public_id: request.product_public_id.clone(),
                expires_at,
                status: ScanTokenStatus::Unused,
                created_at: now,
                used_at: None,
                used_ip: None,
                used_user_agent: None,
                revoked_at: None,
                token_hash: self.token_service.hash_token(&generated.token),
            });
            generated_tokens.push(GeneratedScanTokenUrl {
                token_id: generated.token_id,
                token: generated.token,
                expires_at,
            });
        }

        self.token_repo.save_many(&tokens_to_store).await?;

        Ok(GenerateScanTokensResponse {
            product_public_id: request.product_public_id,
            batch_id,
            tokens: generated_tokens,
        })
    }
}

fn validate_generate_request(request: &GenerateScanTokensRequest) -> Result<(), AppError> {
    if request.product_public_id.trim().is_empty() {
        return Err(AppError::Validation(
            "product_public_id cannot be empty".to_string(),
        ));
    }
    if request.count == 0 || request.count > 1_000 {
        return Err(AppError::Validation(
            "count must be between 1 and 1000".to_string(),
        ));
    }
    if request.ttl_seconds <= 0 {
        return Err(AppError::Validation(
            "ttl_seconds must be greater than 0".to_string(),
        ));
    }
    Ok(())
}

pub struct ConsumeScanTokenUseCase {
    token_repo: Arc<dyn ScanTokenRepository>,
    scan_repo: Arc<dyn ScanEventRepository>,
    token_service: Arc<dyn ScanTokenService>,
}

pub struct ConsumeScanTokenRequest {
    pub product_public_id: String,
    pub token: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

pub struct ConsumeScanTokenResponse {
    pub result: StaticScanResult,
    pub authentic: bool,
    pub product_public_id: String,
}

impl ConsumeScanTokenUseCase {
    pub fn new(
        token_repo: Arc<dyn ScanTokenRepository>,
        scan_repo: Arc<dyn ScanEventRepository>,
        token_service: Arc<dyn ScanTokenService>,
    ) -> Self {
        Self {
            token_repo,
            scan_repo,
            token_service,
        }
    }

    pub async fn execute(
        &self,
        request: ConsumeScanTokenRequest,
    ) -> Result<ConsumeScanTokenResponse, AppError> {
        if request.product_public_id.trim().is_empty() {
            return Err(AppError::Validation("pid is required".to_string()));
        }
        if request.token.trim().is_empty() {
            return Err(AppError::Validation("t is required".to_string()));
        }

        let now = Utc::now();
        let (token_id, result) = match self
            .token_service
            .parse_and_verify_token(&request.product_public_id, &request.token)
        {
            Ok(verified) => match self.token_repo.find_by_id(verified.token_id).await? {
                None => (Some(verified.token_id), StaticScanResult::NotFound),
                Some(token) if token.product_public_id != request.product_public_id => {
                    (Some(token.token_id), StaticScanResult::Invalid)
                }
                Some(token) if token.status == ScanTokenStatus::Revoked => {
                    (Some(token.token_id), StaticScanResult::Revoked)
                }
                Some(token) if token.status == ScanTokenStatus::Used => {
                    (Some(token.token_id), StaticScanResult::Replay)
                }
                Some(token) if token.expires_at < now || verified.expires_at < now => {
                    (Some(token.token_id), StaticScanResult::Expired)
                }
                Some(token) => {
                    let consumed = self
                        .token_repo
                        .consume_if_unused(
                            token.token_id,
                            now,
                            request.ip.clone(),
                            request.user_agent.clone(),
                        )
                        .await?;
                    let result = if consumed {
                        StaticScanResult::Ok
                    } else {
                        StaticScanResult::Replay
                    };
                    (Some(token.token_id), result)
                }
            },
            Err(_) => (None, StaticScanResult::Invalid),
        };

        self.scan_repo
            .save(&ScanEvent {
                id: Uuid::new_v4(),
                tag_id: None,
                token_id,
                tag_uid: String::new(),
                product_public_id: Some(request.product_public_id.clone()),
                received_counter: None,
                verdict: result.to_string(),
                metadata: Some(json!({ "mode": "one_time_tokens" }).to_string()),
                ip: request.ip.clone(),
                user_agent: request.user_agent.clone(),
                created_at: now,
            })
            .await?;

        Ok(ConsumeScanTokenResponse {
            authentic: result == StaticScanResult::Ok,
            result,
            product_public_id: request.product_public_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::scan_tokens::application::ports::ScanTokenRepository;
    use crate::modules::scan_tokens::infrastructure::crypto::hmac_scan_token::HmacScanTokenService;
    use crate::modules::tags::domain::entities::Item;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeItemRepository {
        products: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl ItemRepository for FakeItemRepository {
        async fn save(&self, item: &Item) -> Result<Item, AppError> {
            self.products
                .lock()
                .unwrap()
                .push(item.product_code.clone());
            Ok(item.clone())
        }

        async fn find_by_tag_id(&self, _tag_id: Uuid) -> Result<Option<Item>, AppError> {
            Ok(None)
        }

        async fn list_all(&self) -> Result<Vec<Item>, AppError> {
            Ok(Vec::new())
        }

        async fn exists_by_product_code(&self, product_code: &str) -> Result<bool, AppError> {
            Ok(self
                .products
                .lock()
                .unwrap()
                .iter()
                .any(|value| value == product_code))
        }
    }

    #[derive(Default)]
    struct FakeScanTokenRepository {
        tokens: Mutex<HashMap<Uuid, ScanToken>>,
    }

    #[async_trait]
    impl ScanTokenRepository for FakeScanTokenRepository {
        async fn save_many(&self, tokens: &[ScanToken]) -> Result<(), AppError> {
            let mut store = self.tokens.lock().unwrap();
            for token in tokens {
                store.insert(token.token_id, token.clone());
            }
            Ok(())
        }

        async fn save_batch(&self, batch: &TokenBatch) -> Result<TokenBatch, AppError> {
            Ok(batch.clone())
        }

        async fn find_by_id(&self, token_id: Uuid) -> Result<Option<ScanToken>, AppError> {
            Ok(self.tokens.lock().unwrap().get(&token_id).cloned())
        }

        async fn revoke(
            &self,
            _token_id: Uuid,
            _revoked_at: chrono::DateTime<Utc>,
        ) -> Result<bool, AppError> {
            Ok(false)
        }

        async fn revoke_active_batch_for_tag(
            &self,
            _tag_id: Uuid,
            _revoked_at: chrono::DateTime<Utc>,
        ) -> Result<u64, AppError> {
            Ok(0)
        }

        async fn consume_if_unused(
            &self,
            token_id: Uuid,
            used_at: chrono::DateTime<Utc>,
            _used_ip: Option<String>,
            _used_user_agent: Option<String>,
        ) -> Result<bool, AppError> {
            let mut store = self.tokens.lock().unwrap();
            let Some(token) = store.get_mut(&token_id) else {
                return Ok(false);
            };
            if token.status != ScanTokenStatus::Unused {
                return Ok(false);
            }
            token.status = ScanTokenStatus::Used;
            token.used_at = Some(used_at);
            Ok(true)
        }
    }

    #[tokio::test]
    async fn generate_scan_tokens_creates_batch_and_tokens() {
        let item_repo = Arc::new(FakeItemRepository::default());
        item_repo
            .save(&Item {
                id: Uuid::new_v4(),
                product_code: "SKU-123".to_string(),
                size: None,
                color: None,
                tag_id: Uuid::new_v4(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .await
            .unwrap();

        let usecase = GenerateScanTokensUseCase::new(
            item_repo,
            Arc::new(FakeScanTokenRepository::default()),
            Arc::new(HmacScanTokenService::new("unit-test-secret")),
        );

        let response = usecase
            .execute(GenerateScanTokensRequest {
                product_public_id: "SKU-123".to_string(),
                count: 2,
                ttl_seconds: 60,
            })
            .await
            .unwrap();

        assert_eq!(response.tokens.len(), 2);
    }
}
