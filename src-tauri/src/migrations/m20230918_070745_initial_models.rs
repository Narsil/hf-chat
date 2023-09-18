use crate::entities::model::{Parameters, PromptExample, Prompts};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, ModelTrait, QueryFilter,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        let db = manager.get_connection();

        let falcon = crate::entities::model::ActiveModel {
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
        crate::entities::model::ActiveModel {
            id: Set("meta-llama/Llama-2-7b-chat-hf".into()),
            name: Set("meta-llama/Llama-2-7b-chat-hf".into()),
            parameters: Set(crate::entities::model::Parameters {
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
        let falcon: Option<crate::entities::model::Model> = crate::entities::model::Entity::find()
            .filter(crate::entities::model::Column::Id.eq("tiiuae/falcon-180B-chat".to_string()))
            .one(db)
            .await?;
        if let Some(falcon) = falcon {
            falcon.delete(db).await?;
        }
        let llama: Option<crate::entities::model::Model> = crate::entities::model::Entity::find()
            .filter(
                crate::entities::model::Column::Id.eq("meta-llama/Llama-2-7b-chat-hf".to_string()),
            )
            .one(db)
            .await?;
        if let Some(llama) = llama {
            llama.delete(db).await?;
        }

        Ok(())
    }
}
