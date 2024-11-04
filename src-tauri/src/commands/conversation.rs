use crate::entities::conversation;
use crate::entities::message;
use crate::entities::model;
use crate::State;
use chrono::Utc;
use log::info;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Missing user")]
    MissingUser,

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
    conversation_id: u32,
    user_id: u32,
    content: String,
}

impl From<message::Model> for Message {
    fn from(value: message::Model) -> Self {
        Message {
            content: value.content,
            user_id: value.user_id,
            conversation_id: value.conversation_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Conversation {
    model_id: u32,
    title: String,
    messages: Vec<Message>,
}

impl From<(conversation::Model, Vec<message::Model>)> for Conversation {
    fn from((conv, messages): (conversation::Model, Vec<message::Model>)) -> Self {
        Conversation {
            model_id: conv.model_id,
            title: conv.title,
            messages: messages.into_iter().map(Message::from).collect(),
        }
    }
}

#[tauri::command]
pub async fn create_conversation(
    state: tauri::State<'_, State>,
    modelid: u32,
) -> Result<Conversation, Error> {
    let model_id = modelid;
    let db = &state.db;
    let model = model::Entity::find_by_id(model_id)
        .one(db)
        .await?
        .ok_or(Error::MissingModel(model_id))?;
    let now = Utc::now();
    let conversation = conversation::ActiveModel {
        model_id: Set(model.id),
        title: Set("Conversation".to_owned()),
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
        messages,
        title: conversation.title,
        model_id: conversation.model_id,
    };
    Ok(conversation)
}
