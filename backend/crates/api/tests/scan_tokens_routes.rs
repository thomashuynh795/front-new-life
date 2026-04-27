use api::app::error::AppError;
use api::app::http::{AppState, create_router};
use api::modules::scan_tokens::application::ports::ScanTokenRepository;
use api::modules::scan_tokens::application::scan_tokens::{
    ConsumeScanTokenUseCase, GenerateScanTokensUseCase,
};
use api::modules::scan_tokens::domain::entities::{
    ScanToken, ScanTokenStatus, TokenBatch, TokenBatchStatus,
};
use api::modules::scan_tokens::infrastructure::crypto::hmac_scan_token::HmacScanTokenService;
use api::modules::tags::application::admin::{
    ListCatalogItemsUseCase, ListCatalogTagsUseCase, NextMessagesUseCase, ReconfigureTagUseCase,
    RevokeScanTokenUseCase, RevokeTagUseCase, RotateKeyUseCase,
};
use api::modules::tags::application::ports::{
    AuditEventRepository, ItemRepository, ScanEventRepository, TagRepository,
};
use api::modules::tags::application::provision::EnrollTagUseCase;
use api::modules::tags::application::verify::VerifyTagUseCase;
use api::modules::tags::domain::entities::{AuditEvent, Item, ScanEvent, Tag, TagStatus};
use api::modules::tags::infrastructure::crypto::aes_cmac::AesCmacService;
use async_trait::async_trait;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Default)]
struct InMemoryTagRepository {
    tags: Mutex<HashMap<Uuid, Tag>>,
}

#[async_trait]
impl TagRepository for InMemoryTagRepository {
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
        tag.updated_at = Utc::now();
        Ok(true)
    }

    async fn revoke(&self, tag_id: Uuid) -> Result<(), AppError> {
        if let Some(tag) = self.tags.lock().unwrap().get_mut(&tag_id) {
            tag.status = TagStatus::Revoked;
            tag.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn rotate_key(
        &self,
        tag_id: Uuid,
        new_version: i32,
        reset_counter: bool,
    ) -> Result<(), AppError> {
        if let Some(tag) = self.tags.lock().unwrap().get_mut(&tag_id) {
            tag.key_version = new_version;
            if reset_counter {
                tag.last_counter = None;
            }
            tag.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn reset_counter(&self, tag_id: Uuid) -> Result<(), AppError> {
        if let Some(tag) = self.tags.lock().unwrap().get_mut(&tag_id) {
            tag.last_counter = None;
            tag.updated_at = Utc::now();
        }
        Ok(())
    }
}

#[derive(Default)]
struct InMemoryItemRepository {
    items: Mutex<HashMap<Uuid, Item>>,
}

#[async_trait]
impl ItemRepository for InMemoryItemRepository {
    async fn save(&self, item: &Item) -> Result<Item, AppError> {
        self.items.lock().unwrap().insert(item.id, item.clone());
        Ok(item.clone())
    }

    async fn find_by_tag_id(&self, tag_id: Uuid) -> Result<Option<Item>, AppError> {
        Ok(self
            .items
            .lock()
            .unwrap()
            .values()
            .find(|item| item.tag_id == tag_id)
            .cloned())
    }

    async fn list_all(&self) -> Result<Vec<Item>, AppError> {
        Ok(self.items.lock().unwrap().values().cloned().collect())
    }

    async fn exists_by_product_code(&self, product_code: &str) -> Result<bool, AppError> {
        Ok(self
            .items
            .lock()
            .unwrap()
            .values()
            .any(|item| item.product_code == product_code))
    }
}

#[derive(Default)]
struct InMemoryScanTokenRepository {
    tokens: Mutex<HashMap<Uuid, ScanToken>>,
    batches: Mutex<HashMap<Uuid, TokenBatch>>,
}

#[async_trait]
impl ScanTokenRepository for InMemoryScanTokenRepository {
    async fn save_many(&self, tokens: &[ScanToken]) -> Result<(), AppError> {
        let mut store = self.tokens.lock().unwrap();
        for token in tokens {
            store.insert(token.token_id, token.clone());
        }
        Ok(())
    }

    async fn save_batch(&self, batch: &TokenBatch) -> Result<TokenBatch, AppError> {
        self.batches.lock().unwrap().insert(batch.id, batch.clone());
        Ok(batch.clone())
    }

    async fn find_by_id(&self, token_id: Uuid) -> Result<Option<ScanToken>, AppError> {
        Ok(self.tokens.lock().unwrap().get(&token_id).cloned())
    }

    async fn revoke(
        &self,
        token_id: Uuid,
        revoked_at: chrono::DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let mut store = self.tokens.lock().unwrap();
        let Some(token) = store.get_mut(&token_id) else {
            return Ok(false);
        };
        if token.status == ScanTokenStatus::Used {
            return Ok(false);
        }
        token.status = ScanTokenStatus::Revoked;
        token.revoked_at = Some(revoked_at);
        Ok(true)
    }

    async fn revoke_active_batch_for_tag(
        &self,
        tag_id: Uuid,
        revoked_at: chrono::DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let mut count = 0;
        {
            let mut batches = self.batches.lock().unwrap();
            for batch in batches.values_mut() {
                if batch.tag_id == Some(tag_id) && batch.status == TokenBatchStatus::Active {
                    batch.status = TokenBatchStatus::Revoked;
                    batch.revoked_at = Some(revoked_at);
                    count += 1;
                }
            }
        }

        let mut tokens = self.tokens.lock().unwrap();
        for token in tokens.values_mut() {
            if token.tag_id == Some(tag_id) && token.status == ScanTokenStatus::Unused {
                token.status = ScanTokenStatus::Revoked;
                token.revoked_at = Some(revoked_at);
            }
        }

        Ok(count)
    }

    async fn consume_if_unused(
        &self,
        token_id: Uuid,
        used_at: chrono::DateTime<Utc>,
        used_ip: Option<String>,
        used_user_agent: Option<String>,
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
        token.used_ip = used_ip;
        token.used_user_agent = used_user_agent;
        Ok(true)
    }
}

#[derive(Default)]
struct InMemoryScanEventRepository {
    events: Mutex<Vec<ScanEvent>>,
}

#[async_trait]
impl ScanEventRepository for InMemoryScanEventRepository {
    async fn save(&self, event: &ScanEvent) -> Result<ScanEvent, AppError> {
        self.events.lock().unwrap().push(event.clone());
        Ok(event.clone())
    }
}

#[derive(Default)]
struct InMemoryAuditEventRepository {
    events: Mutex<Vec<AuditEvent>>,
}

#[async_trait]
impl AuditEventRepository for InMemoryAuditEventRepository {
    async fn save(&self, event: &AuditEvent) -> Result<AuditEvent, AppError> {
        self.events.lock().unwrap().push(event.clone());
        Ok(event.clone())
    }
}

struct TestHarness {
    app: axum::Router,
}

async fn build_app() -> TestHarness {
    let tag_repo = Arc::new(InMemoryTagRepository::default());
    let item_repo = Arc::new(InMemoryItemRepository::default());
    let scan_repo = Arc::new(InMemoryScanEventRepository::default());
    let audit_repo = Arc::new(InMemoryAuditEventRepository::default());
    let token_repo = Arc::new(InMemoryScanTokenRepository::default());
    let crypto_service = Arc::new(AesCmacService::new("000102030405060708090A0B0C0D0E0F").unwrap());
    let token_service = Arc::new(HmacScanTokenService::new("test-token-secret"));

    seed_product(item_repo.clone()).await;

    let enroll_usecase = Arc::new(EnrollTagUseCase::new(
        tag_repo.clone(),
        item_repo.clone(),
        audit_repo.clone(),
        token_repo.clone(),
        token_service.clone(),
        3,
        Duration::hours(1).num_seconds(),
    ));

    let state = Arc::new(AppState {
        api_base_url: "https://api.example.com".to_string(),
        admin_key: "admin-secret".to_string(),
        db_wipe_token: Some("wipe-secret".to_string()),
        database_connection: None,
        enroll_usecase: enroll_usecase.clone(),
        verify_usecase: Arc::new(VerifyTagUseCase::new(
            tag_repo.clone(),
            scan_repo.clone(),
            crypto_service.clone(),
        )),
        revoke_usecase: Arc::new(RevokeTagUseCase::new(tag_repo.clone(), audit_repo.clone())),
        rotate_usecase: Arc::new(RotateKeyUseCase::new(tag_repo.clone(), audit_repo.clone())),
        reconfigure_usecase: Arc::new(ReconfigureTagUseCase::new(
            tag_repo.clone(),
            item_repo.clone(),
            audit_repo.clone(),
            enroll_usecase,
            token_repo.clone(),
        )),
        next_messages_usecase: Arc::new(NextMessagesUseCase::new(
            tag_repo.clone(),
            crypto_service.clone(),
        )),
        list_catalog_items_usecase: Arc::new(ListCatalogItemsUseCase::new(
            item_repo.clone(),
            tag_repo.clone(),
        )),
        list_catalog_tags_usecase: Arc::new(ListCatalogTagsUseCase::new(
            item_repo.clone(),
            tag_repo.clone(),
        )),
        revoke_scan_token_usecase: Arc::new(RevokeScanTokenUseCase::new(token_repo.clone())),
        generate_scan_tokens_usecase: Arc::new(GenerateScanTokensUseCase::new(
            item_repo,
            token_repo.clone(),
            token_service.clone(),
        )),
        consume_scan_token_usecase: Arc::new(ConsumeScanTokenUseCase::new(
            token_repo,
            scan_repo,
            token_service,
        )),
    });

    TestHarness {
        app: create_router(state),
    }
}

#[tokio::test]
async fn wipe_database_route_requires_bearer_token() {
    let harness = build_app().await;

    let response = harness
        .app
        .oneshot(
            Request::post("/admin/database/wipe")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wipe_database_route_returns_service_unavailable_without_db_connection() {
    let harness = build_app().await;

    let response = harness
        .app
        .oneshot(
            Request::post("/admin/database/wipe")
                .header("authorization", "Bearer wipe-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

async fn seed_product(item_repo: Arc<InMemoryItemRepository>) {
    item_repo
        .save(&Item {
            id: Uuid::new_v4(),
            product_code: "SKU-123".to_string(),
            size: Some("M".to_string()),
            color: Some("BLACK".to_string()),
            tag_id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn generate_scan_tokens_route_returns_urls() {
    let harness = build_app().await;

    let response = harness
        .app
        .oneshot(
            Request::post("/v1/products/SKU-123/scan-tokens")
                .header("x-admin-key", "admin-secret")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"count": 3, "ttl_seconds": Duration::hours(1).num_seconds()})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = json_body(response).await;
    assert_eq!(body["product_public_id"], "SKU-123");
    assert_eq!(body["tokens"].as_array().unwrap().len(), 3);
    assert!(body["batch_id"].is_string());
}

#[tokio::test]
async fn revoke_scan_token_route_forces_scan_to_return_forbidden() {
    let harness = build_app().await;

    let generate_response = harness
        .app
        .clone()
        .oneshot(
            Request::post("/v1/products/SKU-123/scan-tokens")
                .header("x-admin-key", "admin-secret")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"count": 1, "ttl_seconds": Duration::hours(1).num_seconds()})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let generated_body = json_body(generate_response).await;
    let token_id = generated_body["tokens"][0]["token_id"].as_str().unwrap();
    let token = generated_body["tokens"][0]["token"].as_str().unwrap();

    let revoke_response = harness
        .app
        .clone()
        .oneshot(
            Request::post(format!("/admin/scan-tokens/{token_id}/revoke"))
                .header("x-admin-key", "admin-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke_response.status(), StatusCode::OK);

    let scan_response = harness
        .app
        .oneshot(
            Request::get(format!("/v1/scan?pid=SKU-123&t={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(scan_response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn concurrent_scan_requests_consume_token_only_once() {
    let harness = build_app().await;

    let generate_response = harness
        .app
        .clone()
        .oneshot(
            Request::post("/v1/products/SKU-123/scan-tokens")
                .header("x-admin-key", "admin-secret")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"count": 1, "ttl_seconds": Duration::hours(1).num_seconds()})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let generated_body = json_body(generate_response).await;
    let token = generated_body["tokens"][0]["token"]
        .as_str()
        .unwrap()
        .to_string();

    let path = format!("/v1/scan?pid=SKU-123&t={token}");
    let mut handles: Vec<JoinHandle<StatusCode>> = Vec::new();
    for _ in 0..8 {
        let app = harness.app.clone();
        let path = path.clone();
        handles.push(tokio::spawn(async move {
            app.oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status()
        }));
    }

    let mut ok_count = 0;
    let mut conflict_count = 0;
    for handle in handles {
        match handle.await.unwrap() {
            StatusCode::OK => ok_count += 1,
            StatusCode::CONFLICT => conflict_count += 1,
            status => panic!("unexpected status: {status}"),
        }
    }

    assert_eq!(ok_count, 1);
    assert_eq!(conflict_count, 7);
}

#[tokio::test]
async fn next_messages_then_verify_accepts_three_messages_and_rejects_replay() {
    let harness = build_app().await;
    let tag_id = enroll_dynamic_tag(&harness.app).await;

    let next_messages_response = harness
        .app
        .clone()
        .oneshot(
            Request::post(format!("/admin/tags/{tag_id}/next-messages"))
                .header("x-admin-key", "admin-secret")
                .header("content-type", "application/json")
                .body(Body::from(json!({"count": 3}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(next_messages_response.status(), StatusCode::OK);
    let body = json_body(next_messages_response).await;
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);

    for message in messages {
        let verify_response = harness
            .app
            .clone()
            .oneshot(
                Request::post("/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "tag_uid": body["tag_uid"],
                            "counter": message["counter"],
                            "cmac": message["cmac"],
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(verify_response.status(), StatusCode::OK);
    }

    let replay_response = harness
        .app
        .oneshot(
            Request::post("/verify")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "tag_uid": body["tag_uid"],
                        "counter": messages[1]["counter"],
                        "cmac": messages[1]["cmac"],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(replay_response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn next_messages_respects_explicit_starting_counter() {
    let harness = build_app().await;
    let tag_id = enroll_dynamic_tag(&harness.app).await;

    let response = harness
        .app
        .oneshot(
            Request::post(format!("/admin/tags/{tag_id}/next-messages"))
                .header("x-admin-key", "admin-secret")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"count": 2, "starting_counter": 42}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["messages"][0]["counter"], 42);
    assert_eq!(body["messages"][1]["counter"], 43);
}

#[tokio::test]
async fn concurrent_verify_requests_accept_only_one_of_same_counter_and_cmac() {
    let harness = build_app().await;
    let tag_id = enroll_dynamic_tag(&harness.app).await;
    let next_messages_response = harness
        .app
        .clone()
        .oneshot(
            Request::post(format!("/admin/tags/{tag_id}/next-messages"))
                .header("x-admin-key", "admin-secret")
                .header("content-type", "application/json")
                .body(Body::from(json!({"count": 1}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(next_messages_response).await;
    let payload = json!({
        "tag_uid": body["tag_uid"],
        "counter": body["messages"][0]["counter"],
        "cmac": body["messages"][0]["cmac"],
    })
    .to_string();

    let mut handles: Vec<JoinHandle<StatusCode>> = Vec::new();
    for _ in 0..8 {
        let app = harness.app.clone();
        let payload = payload.clone();
        handles.push(tokio::spawn(async move {
            app.oneshot(
                Request::post("/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
        }));
    }

    let mut ok_count = 0;
    let mut conflict_count = 0;
    for handle in handles {
        match handle.await.unwrap() {
            StatusCode::OK => ok_count += 1,
            StatusCode::CONFLICT => conflict_count += 1,
            status => panic!("unexpected status: {status}"),
        }
    }

    assert_eq!(ok_count, 1);
    assert_eq!(conflict_count, 7);
}

async fn enroll_dynamic_tag(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::post("/admin/tags/enroll")
                .header("x-admin-key", "admin-secret")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "tag_uid": "04AABBCCDD",
                        "product_code": "SKU-123",
                        "size": "M",
                        "color": "BLACK",
                        "mode": "dynamic_cmac",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await["tag_id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn json_body(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}
