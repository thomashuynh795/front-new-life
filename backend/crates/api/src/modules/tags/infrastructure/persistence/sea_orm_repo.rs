use crate::app::error::AppError;
use crate::modules::tags::application::ports::{
    AuditEventRepository, ItemRepository, ScanEventRepository, TagRepository,
};
use crate::modules::tags::domain::entities::{
    AuditEvent, Item, ScanEvent, Tag, TagMode, TagStatus,
};
use async_trait::async_trait;
use database_model::{audit_event, item, scan_event, tag};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use uuid::Uuid;

fn parse_tag_mode(value: &str) -> Result<TagMode, AppError> {
    TagMode::try_from(value).map_err(AppError::Internal)
}

impl TryFrom<tag::Model> for Tag {
    type Error = AppError;

    fn try_from(model: tag::Model) -> Result<Self, Self::Error> {
        Ok(Tag {
            id: model.id,
            tag_uid: model.tag_uid,
            mode: parse_tag_mode(&model.mode)?,
            status: match model.status.as_str() {
                "ACTIVE" => TagStatus::Active,
                "REVOKED" => TagStatus::Revoked,
                _ => return Err(AppError::Internal("Unknown tag status".to_string())),
            },
            key_version: model.key_version,
            last_counter: model.last_counter,
            created_at: model.created_at.into(),
            updated_at: model.updated_at.into(),
        })
    }
}

impl From<item::Model> for Item {
    fn from(model: item::Model) -> Self {
        Item {
            id: model.id,
            product_code: model.product_code,
            size: model.size,
            color: model.color,
            tag_id: model.tag_id,
            created_at: model.created_at.into(),
            updated_at: model.updated_at.into(),
        }
    }
}

pub struct SeaOrmTagRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmTagRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TagRepository for SeaOrmTagRepository {
    async fn save(&self, tag_value: &Tag) -> Result<Tag, AppError> {
        let active_model = tag::ActiveModel {
            id: Set(tag_value.id),
            tag_uid: Set(tag_value.tag_uid.clone()),
            mode: Set(tag_value.mode.to_string()),
            status: Set(tag_value.status.to_string()),
            key_version: Set(tag_value.key_version),
            last_counter: Set(tag_value.last_counter),
            created_at: Set(tag_value.created_at.into()),
            updated_at: Set(tag_value.updated_at.into()),
        };

        let result = tag::Entity::insert(active_model)
            .exec_with_returning(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Tag::try_from(result)
    }

    async fn find_by_uid(&self, tag_uid: &str) -> Result<Option<Tag>, AppError> {
        let result = tag::Entity::find()
            .filter(tag::Column::TagUid.eq(tag_uid))
            .one(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        result.map(Tag::try_from).transpose()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Tag>, AppError> {
        let result = tag::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        result.map(Tag::try_from).transpose()
    }

    async fn list_all(&self) -> Result<Vec<Tag>, AppError> {
        let results = tag::Entity::find()
            .all(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        results.into_iter().map(Tag::try_from).collect()
    }

    async fn update_counter_if_greater(
        &self,
        tag_id: Uuid,
        new_counter: i64,
    ) -> Result<bool, AppError> {
        let result = tag::Entity::update_many()
            .col_expr(tag::Column::LastCounter, Expr::value(Some(new_counter)))
            .col_expr(tag::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(tag::Column::Id.eq(tag_id))
            .filter(
                Condition::any()
                    .add(tag::Column::LastCounter.is_null())
                    .add(tag::Column::LastCounter.lt(new_counter)),
            )
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn revoke(&self, tag_id: Uuid) -> Result<(), AppError> {
        tag::Entity::update_many()
            .col_expr(tag::Column::Status, Expr::value("REVOKED"))
            .col_expr(tag::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(tag::Column::Id.eq(tag_id))
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(())
    }

    async fn rotate_key(
        &self,
        tag_id: Uuid,
        new_version: i32,
        reset_counter: bool,
    ) -> Result<(), AppError> {
        let mut query = tag::Entity::update_many()
            .col_expr(tag::Column::KeyVersion, Expr::value(new_version))
            .col_expr(tag::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(tag::Column::Id.eq(tag_id));

        if reset_counter {
            query = query.col_expr(tag::Column::LastCounter, Expr::value::<Option<i64>>(None));
        }

        query
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(())
    }

    async fn reset_counter(&self, tag_id: Uuid) -> Result<(), AppError> {
        tag::Entity::update_many()
            .col_expr(tag::Column::LastCounter, Expr::value::<Option<i64>>(None))
            .col_expr(tag::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(tag::Column::Id.eq(tag_id))
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(())
    }
}

pub struct SeaOrmItemRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmItemRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ItemRepository for SeaOrmItemRepository {
    async fn save(&self, item_value: &Item) -> Result<Item, AppError> {
        let active_model = item::ActiveModel {
            id: Set(item_value.id),
            product_code: Set(item_value.product_code.clone()),
            size: Set(item_value.size.clone()),
            color: Set(item_value.color.clone()),
            tag_id: Set(item_value.tag_id),
            created_at: Set(item_value.created_at.into()),
            updated_at: Set(item_value.updated_at.into()),
        };

        let result = item::Entity::insert(active_model)
            .exec_with_returning(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(Item::from(result))
    }

    async fn find_by_tag_id(&self, tag_id: Uuid) -> Result<Option<Item>, AppError> {
        let result = item::Entity::find()
            .filter(item::Column::TagId.eq(tag_id))
            .one(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(result.map(Item::from))
    }

    async fn list_all(&self) -> Result<Vec<Item>, AppError> {
        let results = item::Entity::find()
            .all(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(results.into_iter().map(Item::from).collect())
    }

    async fn exists_by_product_code(&self, product_code: &str) -> Result<bool, AppError> {
        let result = item::Entity::find()
            .filter(item::Column::ProductCode.eq(product_code))
            .one(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(result.is_some())
    }
}

pub struct SeaOrmScanEventRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmScanEventRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ScanEventRepository for SeaOrmScanEventRepository {
    async fn save(&self, event: &ScanEvent) -> Result<ScanEvent, AppError> {
        let active_model = scan_event::ActiveModel {
            id: Set(event.id),
            tag_id: Set(event.tag_id),
            token_id: Set(event.token_id),
            tag_uid: Set(event.tag_uid.clone()),
            product_public_id: Set(event.product_public_id.clone()),
            received_counter: Set(event.received_counter),
            verdict: Set(event.verdict.clone()),
            metadata: Set(event.metadata.clone()),
            ip: Set(event.ip.clone()),
            user_agent: Set(event.user_agent.clone()),
            created_at: Set(event.created_at.into()),
        };

        scan_event::Entity::insert(active_model)
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(event.clone())
    }
}

pub struct SeaOrmAuditEventRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmAuditEventRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AuditEventRepository for SeaOrmAuditEventRepository {
    async fn save(&self, event: &AuditEvent) -> Result<AuditEvent, AppError> {
        let active_model = audit_event::ActiveModel {
            id: Set(event.id),
            tag_id: Set(event.tag_id),
            event_type: Set(event.event_type.clone()),
            metadata: Set(event.metadata.clone()),
            created_at: Set(event.created_at.into()),
        };

        audit_event::Entity::insert(active_model)
            .exec(&*self.db)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        Ok(event.clone())
    }
}
