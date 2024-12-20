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

#[derive(Serialize)]
pub struct ConvData {
    messages: Vec<message::Model>,
    users: Vec<user::Model>,
}

#[tauri::command]
pub async fn get_messages(
    state: tauri::State<'_, State>,
    conversationid: u32,
) -> Result<ConvData, Error> {
    let db = &state.db;
    let conversation = conversation::Entity::find_by_id(conversationid)
        .one(db)
        .await?
        .ok_or(Error::MissingModel(conversationid))?;
    let messages: Vec<message::Model> = message::Entity::find()
        .filter(message::Column::ConversationId.eq(conversation.id))
        .all(db)
        .await?;
    // TODO Get only the users from the conversation.
    // Add a link table
    let users: Vec<user::Model> = user::Entity::find().all(db).await?;
    info!(
        "Got {} messages for conv {}",
        messages.len(),
        conversation.id
    );
    Ok(ConvData { messages, users })
}
