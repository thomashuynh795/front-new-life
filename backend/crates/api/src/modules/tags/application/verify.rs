use crate::app::error::AppError;
use crate::modules::tags::application::ports::{CryptoService, ScanEventRepository, TagRepository};
use crate::modules::tags::application::provision::{build_dynamic_message, normalize_tag_uid};
use crate::modules::tags::domain::entities::{ScanEvent, ScanVerdict, TagMode, TagStatus};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub struct VerifyTagUseCase {
    tag_repo: Arc<dyn TagRepository>,
    scan_repo: Arc<dyn ScanEventRepository>,
    crypto_service: Arc<dyn CryptoService>,
}

pub struct VerifyRequest {
    pub tag_uid: String,
    pub counter: i64,
    pub cmac: String,
}

#[derive(Debug)]
pub struct VerifyResponse {
    pub verdict: ScanVerdict,
    pub product_info: Option<ProductInfo>,
}

#[derive(Debug, Clone)]
pub struct ProductInfo {
    pub tag_id: Uuid,
}

impl VerifyTagUseCase {
    pub fn new(
        tag_repo: Arc<dyn TagRepository>,
        scan_repo: Arc<dyn ScanEventRepository>,
        crypto_service: Arc<dyn CryptoService>,
    ) -> Self {
        Self {
            tag_repo,
            scan_repo,
            crypto_service,
        }
    }

    pub async fn execute(&self, request: VerifyRequest) -> Result<VerifyResponse, AppError> {
        let normalized_uid = normalize_tag_uid(&request.tag_uid)?;
        let tag = self.tag_repo.find_by_uid(&normalized_uid).await?;
        let mut verdict = ScanVerdict::TagNotFound;
        let mut tag_id = None;

        if let Some(tag) = tag {
            tag_id = Some(tag.id);
            verdict = if tag.status == TagStatus::Revoked {
                ScanVerdict::TagRevoked
            } else if tag.mode != TagMode::DynamicCmac {
                ScanVerdict::InvalidSignature
            } else {
                let signature = hex::decode(&request.cmac).unwrap_or_default();
                if signature.is_empty() {
                    ScanVerdict::InvalidSignature
                } else {
                    let message = build_dynamic_message(&normalized_uid, request.counter)?;
                    let valid = self
                        .crypto_service
                        .verify_cmac(tag.key_version, &normalized_uid, &message, &signature)
                        .await?;
                    if !valid {
                        ScanVerdict::InvalidSignature
                    } else if self
                        .tag_repo
                        .update_counter_if_greater(tag.id, request.counter)
                        .await?
                    {
                        ScanVerdict::Valid
                    } else {
                        ScanVerdict::ReplayDetected
                    }
                }
            };
        }

        self.scan_repo
            .save(&ScanEvent {
                id: Uuid::new_v4(),
                tag_id,
                token_id: None,
                tag_uid: normalized_uid.clone(),
                product_public_id: None,
                received_counter: Some(request.counter),
                verdict: verdict.to_string(),
                metadata: Some(json!({ "mode": "dynamic_cmac" }).to_string()),
                ip: None,
                user_agent: None,
                created_at: Utc::now(),
            })
            .await?;

        Ok(VerifyResponse {
            verdict: verdict.clone(),
            product_info: tag_id.map(|tag_id| ProductInfo { tag_id }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::tags::application::ports::{CryptoService, TagRepository};
    use crate::modules::tags::domain::entities::{Tag, TagMode, TagStatus};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeTagRepository {
        tags: Mutex<HashMap<Uuid, Tag>>,
    }

    #[async_trait]
    impl TagRepository for FakeTagRepository {
        async fn save(&self, tag: &Tag) -> Result<Tag, AppError> {
            self.tags.lock().unwrap().insert(tag.id, tag.clone());
            Ok(tag.clone())
        }

        async fn find_by_uid(&self, tag_uid: &str) -> Result<Option<Tag>, AppError> {
            Ok(self
                .tags
                .lock()
                .unwrap()
                .values()
                .find(|tag| tag.tag_uid == tag_uid)
                .cloned())
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<Tag>, AppError> {
            Ok(self.tags.lock().unwrap().get(&id).cloned())
        }

        async fn list_all(&self) -> Result<Vec<Tag>, AppError> {
            Ok(self.tags.lock().unwrap().values().cloned().collect())
        }

        async fn update_counter_if_greater(
            &self,
            tag_id: Uuid,
            new_counter: i64,
        ) -> Result<bool, AppError> {
            let mut tags = self.tags.lock().unwrap();
            let Some(tag) = tags.get_mut(&tag_id) else {
                return Ok(false);
            };
            if tag.last_counter.unwrap_or(0) >= new_counter {
                return Ok(false);
            }
            tag.last_counter = Some(new_counter);
            Ok(true)
        }

        async fn revoke(&self, _tag_id: Uuid) -> Result<(), AppError> {
            Ok(())
        }

        async fn rotate_key(
            &self,
            _tag_id: Uuid,
            _new_version: i32,
            _reset_counter: bool,
        ) -> Result<(), AppError> {
            Ok(())
        }

        async fn reset_counter(&self, _tag_id: Uuid) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeScanEventRepository;

    #[async_trait]
    impl ScanEventRepository for FakeScanEventRepository {
        async fn save(&self, event: &ScanEvent) -> Result<ScanEvent, AppError> {
            Ok(event.clone())
        }
    }

    struct FakeCryptoService;

    #[async_trait]
    impl CryptoService for FakeCryptoService {
        async fn verify_cmac(
            &self,
            _key_version: i32,
            _tag_uid: &str,
            _message: &[u8],
            signature: &[u8],
        ) -> Result<bool, AppError> {
            Ok(signature == [0xAA, 0xBB, 0xCC])
        }

        async fn generate_cmac(
            &self,
            _key_version: i32,
            _tag_uid: &str,
            _message: &[u8],
        ) -> Result<Vec<u8>, AppError> {
            Ok(vec![0xAA, 0xBB, 0xCC])
        }
    }

    #[tokio::test]
    async fn verify_marks_replay_when_counter_is_reused() {
        let tag = Tag {
            id: Uuid::new_v4(),
            tag_uid: "04AABBCCDD".to_string(),
            mode: TagMode::DynamicCmac,
            status: TagStatus::Active,
            key_version: 1,
            last_counter: Some(3),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let repo = Arc::new(FakeTagRepository {
            tags: Mutex::new(HashMap::from([(tag.id, tag)])),
        });
        let usecase = VerifyTagUseCase::new(
            repo,
            Arc::new(FakeScanEventRepository),
            Arc::new(FakeCryptoService),
        );

        let response = usecase
            .execute(VerifyRequest {
                tag_uid: "04AABBCCDD".to_string(),
                counter: 3,
                cmac: "AABBCC".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(response.verdict, ScanVerdict::ReplayDetected);
    }
}
