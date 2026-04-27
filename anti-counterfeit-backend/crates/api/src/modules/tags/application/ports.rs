use crate::app::error::AppError;
use crate::modules::tags::domain::entities::{AuditEvent, Item, ScanEvent, Tag};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait TagRepository: Send + Sync {
    async fn save(&self, tag: &Tag) -> Result<Tag, AppError>;
    async fn find_by_uid(&self, tag_uid: &str) -> Result<Option<Tag>, AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Tag>, AppError>;
    async fn list_all(&self) -> Result<Vec<Tag>, AppError>;
    async fn update_counter_if_greater(
        &self,
        tag_id: Uuid,
        new_counter: i64,
    ) -> Result<bool, AppError>;
    async fn revoke(&self, tag_id: Uuid) -> Result<(), AppError>;
    async fn rotate_key(
        &self,
        tag_id: Uuid,
        new_version: i32,
        reset_counter: bool,
    ) -> Result<(), AppError>;
    async fn reset_counter(&self, tag_id: Uuid) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait ItemRepository: Send + Sync {
    async fn save(&self, item: &Item) -> Result<Item, AppError>;
    async fn find_by_tag_id(&self, tag_id: Uuid) -> Result<Option<Item>, AppError>;
    async fn list_all(&self) -> Result<Vec<Item>, AppError>;
    async fn exists_by_product_code(&self, product_code: &str) -> Result<bool, AppError>;
}

#[async_trait::async_trait]
pub trait ScanEventRepository: Send + Sync {
    async fn save(&self, event: &ScanEvent) -> Result<ScanEvent, AppError>;
}

#[async_trait::async_trait]
pub trait AuditEventRepository: Send + Sync {
    async fn save(&self, event: &AuditEvent) -> Result<AuditEvent, AppError>;
}

#[async_trait::async_trait]
pub trait CryptoService: Send + Sync {
    async fn verify_cmac(
        &self,
        key_version: i32,
        tag_uid: &str,
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool, AppError>;

    async fn generate_cmac(
        &self,
        key_version: i32,
        tag_uid: &str,
        message: &[u8],
    ) -> Result<Vec<u8>, AppError>;
}
