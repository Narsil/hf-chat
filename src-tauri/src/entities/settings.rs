use sea_orm::entity::prelude::*;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, FromJsonQueryResult)]
#[serde(transparent)]
pub struct CustomPrompts {
    pub prompts: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "settings")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub share_conversations_with_model_authors: bool,
    pub ethics_model_accepted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub active_model: String,
    pub search_enabled: bool,
    pub custom_prompts: CustomPrompts,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::model::Entity",
        from = "Column::ActiveModel",
        to = "super::model::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Model,
}

impl Related<super::model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Model.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
