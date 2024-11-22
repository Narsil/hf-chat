use crate::entities::conversation;
use crate::entities::message;
use crate::entities::model;
use crate::entities::user;
use crate::State;
use chrono::{DateTime, Utc};
use log::info;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Missing user {0}")]
    MissingUser(u32),

    #[error("Missing model {0}")]
    MissingModel(u32),

    #[error("Db error {0}")]
    DbError(#[from] sea_orm::DbErr),
}

// we must manually implement serde::Serialize
impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    created: DateTime<Utc>,
    conversation_id: u32,
    user_id: u32,
    content: String,
}

impl From<message::Model> for Message {
    fn from(value: message::Model) -> Self {
        Message {
            created: value.created_at,
            content: value.content,
            user_id: value.user_id,
            conversation_id: value.conversation_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Conversation {
    pub id: u32,
    pub model_id: u32,
    pub user_id: u32,
    pub title: String,
    pub profile: String,
    pub messages: Vec<Message>,
}

// impl From<(conversation::Model, Vec<message::Model>)> for Conversation {
//     fn from((conv, messages): (conversation::Model, Vec<message::Model>)) -> Self {
//         Conversation {
//             id: conv.id,
//             model_id: conv.model_id,
//             title: conv.title,
//             messages: messages.into_iter().map(Message::from).collect(),
//         }
//     }
// }

#[tauri::command]
pub async fn create_conversation(
    state: tauri::State<'_, State>,
    modelid: u32,
) -> Result<Conversation, Error> {
    let model_id = modelid;
    let db = &state.db;
    let (model, user) = model::Entity::find_by_id(model_id)
        .find_also_related(user::Entity)
        .one(db)
        .await?
        .ok_or(Error::MissingModel(model_id))?;
    let user = user.expect("Models have linked user");
    let now = Utc::now();
    let conversation = conversation::ActiveModel {
        model_id: Set(model.id),
        title: Set(format!("{}", user.name)),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let conversation = conversation.insert(db).await?;
    let messages: Vec<message::Model> = message::Entity::find()
        .filter(message::Column::ConversationId.eq(conversation.id))
        .all(db)
        .await?;

    let messages = messages.into_iter().map(Message::from).collect();

    let conversation = Conversation {
        id: conversation.id,
        messages,
        profile: user.profile,
        title: conversation.title,
        model_id: conversation.model_id,
        user_id: user.id,
    };
    Ok(conversation)
}

#[tauri::command]
pub async fn new_message(
    state: tauri::State<'_, State>,
    conversationid: u32,
    content: String,
    authorid: u32,
) -> Result<(), Error> {
    let db = &state.db;
    let user = user::Entity::find_by_id(authorid)
        .one(db)
        .await?
        .ok_or(Error::MissingUser(authorid))?;
    let conversation = conversation::Entity::find_by_id(conversationid)
        .one(db)
        .await?
        .ok_or(Error::MissingModel(conversationid))?;
    let has_messages = message::Entity::find()
        .filter(message::Column::ConversationId.eq(conversation.id))
        .count(db)
        .await?;
    if has_messages == 0 {
        let new_title = content.split(" ").take(5).collect::<Vec<&str>>().join(" ");
        let mut conv: conversation::ActiveModel = conversation.clone().into();
        conv.title = Set(new_title);
        conv.update(db).await.ok();
    }
    //     });
    // }
    //     let db = state.db.clone();
    //     let content = content.clone();
    //     tokio::spawn(async move{
    //         let messages = vec![api::Message{
    //             role: api::Role::User,
    //             content: format!("You are a summarization AI. You'll never answer a user's question directly, but instead summarize the user's request into a single short sentence of four words or less. Always start your answer with an emoji relevant to the summary. The content is `{content}`.")
    //         }];
    //         let url = "https://api-inference.huggingface.co/models/meta-llama/Meta-Llama-3-8B-Instruct/v1/chat/completions";
    //         let mut new_title = String::new();
    //         if let Ok(mut newstream) = api::query(url.to_string(), messages).await{
    //             while let Ok(Some(chunk)) = newstream.next().await{
    //                 new_title.push_str(&chunk);

    //             }
    //         }
    // if let Ok(Some(conversation)) = conversation::Entity::find_by_id(conversationid)
    //     .one(&db)
    //     .await{
    //         let mut conv: conversation::ActiveModel = conversation.into();
    //         conv.title = Set(new_title.to_string());
    //         conv.update(&db).await.ok();
    // }
    //     });
    // }
    let now = Utc::now();
    let message = message::ActiveModel {
        conversation_id: Set(conversation.id),
        user_id: Set(user.id),
        content: Set(content),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let message = message.insert(db).await?;
    info!(
        "Inserted new mesage {:?} Conv: {:?} user {:?}",
        message.id, conversation.id, user.id
    );
    Ok(())
}

#[tauri::command]
pub async fn get_messages(
    state: tauri::State<'_, State>,
    conversationid: u32,
) -> Result<Vec<message::Model>, Error> {
    let db = &state.db;
    let conversation = conversation::Entity::find_by_id(conversationid)
        .one(db)
        .await?
        .ok_or(Error::MissingModel(conversationid))?;
    let messages: Vec<message::Model> = message::Entity::find()
        .filter(message::Column::ConversationId.eq(conversation.id))
        .all(db)
        .await?;
    info!(
        "Got {} messages for conv {}",
        messages.len(),
        conversation.id
    );
    Ok(messages)
}
