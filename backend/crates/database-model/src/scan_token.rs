use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "scan_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub token_id: Uuid,

    pub batch_id: Option<Uuid>,

    pub tag_id: Option<Uuid>,

    pub product_public_id: String,

    pub expires_at: DateTimeWithTimeZone,

    pub status: String,

    pub created_at: DateTimeWithTimeZone,

    pub used_at: Option<DateTimeWithTimeZone>,

    pub used_ip: Option<String>,

    pub used_user_agent: Option<String>,

    pub revoked_at: Option<DateTimeWithTimeZone>,

    #[sea_orm(unique)]
    pub token_hash: Vec<u8>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    ScanEvents,
    TokenBatch,
    Tag,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::ScanEvents => Entity::has_many(super::scan_event::Entity).into(),
            Self::TokenBatch => Entity::belongs_to(super::token_batch::Entity)
                .from(Column::BatchId)
                .to(super::token_batch::Column::Id)
                .on_delete(ForeignKeyAction::SetNull)
                .into(),
            Self::Tag => Entity::belongs_to(super::tag::Entity)
                .from(Column::TagId)
                .to(super::tag::Column::Id)
                .on_delete(ForeignKeyAction::SetNull)
                .into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}
