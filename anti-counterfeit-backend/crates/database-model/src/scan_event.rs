use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "scan_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
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

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Tag,
    ScanToken,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Tag => Entity::belongs_to(super::tag::Entity)
                .from(Column::TagId)
                .to(super::tag::Column::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .into(),
            Self::ScanToken => Entity::belongs_to(super::scan_token::Entity)
                .from(Column::TokenId)
                .to(super::scan_token::Column::TokenId)
                .on_delete(ForeignKeyAction::SetNull)
                .into(),
        }
    }
}

impl Related<super::tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tag.def()
    }
}

impl Related<super::scan_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ScanToken.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
