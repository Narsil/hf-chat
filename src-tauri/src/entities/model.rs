use sea_orm::entity::prelude::*;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, FromJsonQueryResult)]
#[serde(transparent)]
pub struct Prompts {
    pub prompts: Vec<PromptExample>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PromptExample {
    pub title: String,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromJsonQueryResult)]
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
    pub internal_id: i32,
    // To correspond to UI expectations
    pub id: String,
    pub name: String,
    pub website_url: String,
    pub dataset_name: String,
    pub display_name: String,
    pub description: String,
    pub prompt_examples: Prompts,
    pub parameters: Parameters,
    pub preprompt: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::conversation::Entity")]
    Conversation,
}

impl Related<super::conversation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Conversation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
