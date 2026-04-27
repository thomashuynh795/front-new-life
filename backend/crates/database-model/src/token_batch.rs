use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "token_batches")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    pub tag_id: Option<Uuid>,

    pub product_public_id: String,

    pub status: String,

    pub expires_at: DateTimeWithTimeZone,

    pub created_at: DateTimeWithTimeZone,

    pub revoked_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Tag,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Tag => Entity::belongs_to(super::tag::Entity)
                .from(Column::TagId)
                .to(super::tag::Column::Id)
                .on_delete(ForeignKeyAction::SetNull)
                .into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}
