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
        // Replace the sample below with your own migration scripts
        let db = manager.get_connection();

        let falcon = ActiveModel {
            id: Set("tiiuae/falcon-180B-chat".into()),
            name: Set("tiiuae/falcon-180B-chat".into()),
            website_url: Set("https://api-inference.huggingface.co/models/tiiuae/falcon-180B-chat".into()),
            dataset_name: Set("OpenAssistant/oasst1".into()),
            display_name: Set("tiiuae/falcon-180B-chat".into()),
            description: Set("A good alternative to ChatGPT".into()),
            prompt_examples: Set(Prompts{prompts: vec![PromptExample{ title: "Write an email from bullet list".into(), prompt: "As a restaurant owner, write a professional email to the supplier to get these products every week: \n\n- Wine (x10)\n- Eggs (x24)\n- Bread (x12)".into() }, ]}),
            parameters: Set(Parameters {
                temperature: 0.9,
                truncate: 1000,
                max_new_tokens: 1024,
                stop: vec!["<|endoftext|>".into(), "Falcon:".into(), "User:".into()],
                top_p: 0.95,
                repetition_penalty: 1.2,
                top_k: 50,
                return_full_text: false,
            }),
            preprompt: Set("".into())
        };
        // Insert fails to retrieve back id somehow.
        falcon.insert(db).await.ok();
        ActiveModel {
            id: Set("karpathy/tinyllamas".into()),
            name: Set("karpathy/tinyllamas".into()),
            parameters: Set(Parameters {
                temperature: 0.9,
                truncate: 1000,
                max_new_tokens: 1024,
                stop: vec!["<|endoftext|>".into(), "Falcon:".into(), "User:".into()],
                top_p: 0.95,
                repetition_penalty: 1.2,
                top_k: 50,
                return_full_text: false,
            }),
            website_url: Set("https://huggingface.co/karpathy/tinyllamas".into()),
            dataset_name: Set("OpenAssistant/oasst1".into()),
            display_name: Set("karpathy/tinyllamas".into()),
            description: Set("A tiny simple story model".into()),
            prompt_examples: Set(Prompts {
                prompts: vec![PromptExample {
                    title: "Write a kid story".into(),
                    prompt: " Once upon a time".into(),
                }],
            }),
            preprompt: Set("".into()),
        }
        .insert(db)
        .await
        .ok();
        #[cfg(not(mobile))]
        ActiveModel {
            id: Set("meta-llama/Llama-2-7b-chat-hf".into()),
            name: Set("meta-llama/Llama-2-7b-chat-hf".into()),
            parameters: Set(Parameters {
                temperature: 0.9,
                truncate: 1000,
                max_new_tokens: 1024,
                stop: vec!["<|endoftext|>".into(), "Falcon:".into(), "User:".into()],
                top_p: 0.95,
                repetition_penalty: 1.2,
                top_k: 50,
                return_full_text: false,
            }),
            website_url: Set("https://api-inference.huggingface.co/models/meta-llama/Llama-2-7b-chat-hf".into()),
            dataset_name: Set("OpenAssistant/oasst1".into()),
            display_name: Set("meta-llama/Llama-2-7b-chat-hf".into()),
            description: Set("A good alternative to ChatGPT".into()),
            prompt_examples: Set(Prompts{prompts: vec![PromptExample{ title: "Write an email from bullet list".into(), prompt: "As a restaurant owner, write a professional email to the supplier to get these products every week: \n\n- Wine (x10)\n- Eggs (x24)\n- Bread (x12)".into() }, ]}),
            preprompt: Set("".into())
        }
        .insert(db)
        .await.ok();
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let falcon: Option<Model> = Entity::find()
            .filter(Column::Id.eq("tiiuae/falcon-180B-chat".to_string()))
            .one(db)
            .await?;
        if let Some(falcon) = falcon {
            falcon.delete(db).await?;
        }

        let tinyllama: Option<Model> = Entity::find()
            .filter(Column::Id.eq("karpathy/tinyllamas".to_string()))
            .one(db)
            .await?;
        if let Some(tinyllama) = tinyllama {
            tinyllama.delete(db).await?;
        }

        #[cfg(not(mobile))]
        {
            let llama: Option<Model> = Entity::find()
                .filter(Column::Id.eq("meta-llama/Llama-2-7b-chat-hf".to_string()))
                .one(db)
                .await?;
            if let Some(llama) = llama {
                llama.delete(db).await?;
            }
        }

        Ok(())
    }
}
