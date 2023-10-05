#![allow(unused_variables)]
use sea_orm::entity::prelude::*;
use sea_orm::FromJsonQueryResult;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, ModelTrait, QueryFilter,
};
use sea_orm_migration::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(DeriveMigrationName)]
pub struct Migration;

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
    // #[sea_orm(has_many = "super::conversation::Entity")]
    // Conversation,
}
//
// impl Related<super::conversation::Entity> for Entity {
//     fn to() -> RelationDef {
//         Relation::Conversation.def()
//     }
// }

impl ActiveModelBehavior for ActiveModel {}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        #[cfg(not(mobile))]
        {
            let db = manager.get_connection();
            ActiveModel {
            id: Set("microsoft/phi-1_5".into()),
            name: Set("microsoft/phi-1_5".into()),
            parameters: Set(Parameters {
                temperature: 0.9,
                truncate: 1000,
                max_new_tokens: 1024,
                stop: vec!["\n".into(), "Alice:".into(), "Bob:".into()],
                top_p: 0.95,
                repetition_penalty: 1.2,
                top_k: 50,
                return_full_text: false,
            }),
            website_url: Set("https://huggingface.co/microsoft/phi-1_5".into()),
            dataset_name: Set("OpenAssistant/oasst1".into()),
            display_name: Set("microsoft/phi-1_5".into()),
            description: Set("A strong but small model".into()),
            prompt_examples: Set(Prompts {
                prompts: vec![PromptExample {
                    title: "Analogy".into(),
                    prompt: "Write a detailed analogy between mathematics and a lighthouse.".into(),
                }, PromptExample {
                    title: "Chat".into(),
                    prompt: "Alice: I don't know why, I'm struggling to maintain focus while studying. Any suggestions?".into(),
                }],
            }),
            preprompt: Set("".into()),
        }
        .insert(db)
        .await
        .ok();
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        #[cfg(not(mobile))]
        {
            let tinyllama: Option<Model> = Entity::find()
                .filter(Column::Id.eq("karpathy/tinyllamas".to_string()))
                .one(db)
                .await?;
            if let Some(tinyllama) = tinyllama {
                tinyllama.delete(db).await?;
            }
        }
        Ok(())
    }
}
