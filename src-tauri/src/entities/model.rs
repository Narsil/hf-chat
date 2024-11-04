use sea_orm::entity::prelude::*;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize, FromJsonQueryResult)]
pub struct Parameters {
    pub temperature: f32,
    pub truncate: usize,
    pub max_new_tokens: usize,
    pub stop: Vec<String>,
    pub top_p: f32,
    pub top_k: usize,
    pub repetition_penalty: f32,
    pub return_full_text: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, DeriveEntityModel)]
#[sea_orm(table_name = "model")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub name: String,
    pub profile: String,
    pub endpoint: String,
    pub parameters: Parameters,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
//
// impl RelationTrait for Relation {
//     fn def(&self) -> RelationDef {
//         match self {}
//     }
// }
//
// impl Related<super::user::Entity> for Entity {
//     fn to() -> RelationDef {}
// }

impl ActiveModelBehavior for ActiveModel {}
