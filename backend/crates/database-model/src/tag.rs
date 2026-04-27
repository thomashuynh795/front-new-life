use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    #[sea_orm(unique)]
    pub tag_uid: String,

    pub mode: String,

    pub status: String,

    pub key_version: i32,

    pub last_counter: Option<i64>,

    pub created_at: DateTimeWithTimeZone,

    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Item,
    ScanEvents,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Item => Entity::has_one(super::item::Entity).into(),
            Self::ScanEvents => Entity::has_many(super::scan_event::Entity).into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}
