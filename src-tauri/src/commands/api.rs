use crate::entities::{conversation, message, model, user};
use crate::State;
use ::reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Response,
};
use chrono::Utc;
use core::str;
use log::{debug, error, info};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, thiserror::Error)]
#[cfg_attr(test, derive(Serialize))]
#[error("{error}")]
pub struct SseError {
    error: InnerError,
}

#[derive(Debug, Deserialize, thiserror::Error)]
#[cfg_attr(test, derive(Serialize))]
pub enum InnerError {
    #[serde(rename = "Authorization header is correct, but the token seems invalid")]
    #[error("Invalid token")]
    InvalidToken,

    #[serde(untagged)]
    #[error("{0}")]
    Default(String),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Reqwest(#[from] ::reqwest::Error),

    #[error("Conversation {0} is missing")]
    MissingConversation(u32),

    #[error("Db error {0}")]
    DbError(#[from] sea_orm::DbErr),

    #[error("Invalid chunk error {0}")]
    InvalidChunkError(String),

    #[error("Sse Error {0}")]
    SseError(SseError),

    #[error("Invalid Token")]
    InvalidToken,

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    Local(#[from] crate::commands::local::Error),
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    fn from_db(messages: Vec<message::Model>) -> Vec<Self> {
        let mut role = Role::Assistant;
        let mut last_user = None;
        let mut last_message = None;
        let mut newmessages = Vec::with_capacity(messages.len());
        for message in messages {
            if Some(message.user_id) != last_user {
                if let Some(last_message) = last_message.take() {
                    newmessages.push(last_message);
                }
                role = match role {
                    Role::User => Role::Assistant,
                    Role::Assistant => Role::User,
                };
                last_message = Some(Message {
                    role,
                    content: message.content.clone(),
                });
                last_user = Some(message.user_id);
            } else {
                last_message.as_mut().map(|m| {
                    m.content.push('\n');
                    m.content.push_str(&message.content);
                });
            }
        }
        if let Some(last_message) = last_message.take() {
            newmessages.push(last_message);
        }
        newmessages
    }
}

#[derive(Serialize)]
pub struct Payload {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    max_tokens: usize,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    delta: Message,
}

#[derive(Debug, Deserialize)]
pub struct Chunk {
    choices: Vec<Choice>,
}

pub enum Stream {
    Api(Api),
    Local(crate::commands::local::Stream),
}

impl Stream {
    pub async fn next(&mut self) -> Result<Option<String>, Error> {
        match self {
            Stream::Api(api) => api.next().await,
            Stream::Local(local) => Ok(local.next().await),
        }
    }
}

pub struct Api {
    res: Response,
    leftover: Vec<u8>,
}

pub async fn query(url: String, messages: Vec<Message>, token: &str) -> Result<Api, Error> {
    info!("Query {url} {} messages", messages.len());
    let client = ::reqwest::Client::new();
    let model = "tgi".to_string();

    let stream = true;
    let max_tokens = 1024;
    let temperature = 0.0;
    let payload = Payload {
        model,
        messages,
        stream,
        max_tokens,
        temperature,
    };

    let res = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&payload)
        .send()
        .await?;
    debug!("Client response received");
    // let res = res.error_for_status()?;
    return Ok(Api {
        res,
        leftover: vec![],
    });
}

impl Api {
    pub async fn next(&mut self) -> Result<Option<String>, Error> {
        if let Some(chunk) = self.res.chunk().await? {
            let mut content = String::new();

            for (i, mut subchunk) in chunk.split(|&c| c == b'\n').enumerate() {
                if i == 0 && !self.leftover.is_empty() {
                    self.leftover.extend(subchunk);
                    subchunk = &self.leftover[..];
                }
                if subchunk.is_empty() {
                    // Do nothing
                } else if subchunk.starts_with(b"data: ") {
                    if subchunk == b"data: [DONE]" {
                        continue;
                    }
                    if let Ok(parsed) =
                        serde_json::from_slice::<Chunk>(&subchunk[b"data: ".len()..])
                    {
                        let msg = &parsed.choices[0].delta.content;
                        content.push_str(msg);
                    } else {
                        let owned = subchunk.to_owned();
                        self.leftover.extend(owned);
                    }
                } else if let Ok(parsed) = serde_json::from_slice::<SseError>(&subchunk) {
                    error!("Chunk Error {parsed:?}");
                    return Err(Error::SseError(parsed));
                } else {
                    return Err(Error::InvalidChunkError(
                        String::from_utf8_lossy(&subchunk).into(),
                    ));
                }
            }
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }
}

#[tauri::command]
pub async fn get_chunk(
    state: tauri::State<'_, State>,
    conversationid: u32,
) -> Result<Option<String>, Error> {
    let db = &state.db;
    let (_conversation, model): (conversation::Model, Option<model::Model>) =
        conversation::Entity::find_by_id(conversationid)
            .find_also_related(model::Entity)
            .one(db)
            .await?
            .ok_or(Error::MissingConversation(conversationid))?;
    let model = model.expect("Associated model");
    let mut stream = state.stream.lock().await;
    let chunk = if let Some(ref mut stream) = &mut *stream {
        stream.next().await?
    } else {
        let messages: Vec<message::Model> = message::Entity::find()
            .filter(message::Column::ConversationId.eq(conversationid))
            .all(db)
            .await?;

        let url = model.endpoint;
        if url.starts_with("https://") {
            let messages = Message::from_db(messages);
            let cache = &state.cache;
            let token = cache.token().expect("Expected token");
            let mut newstream = query(url, messages, &token).await?;
            match newstream.next().await {
                Ok(chunk) => {
                    *stream = Some(Stream::Api(newstream));
                    chunk
                }
                Err(Error::SseError(SseError {
                    error: InnerError::InvalidToken,
                })) => {
                    error!("Invalid token, deleting it");
                    std::fs::remove_file(cache.token_path())?;
                    return Err(Error::InvalidToken);
                }
                Err(err) => {
                    return Err(err);
                }
            }
        } else {
            let model_id = url;
            let mut newstream = crate::commands::local::local_stream(model_id, messages).await?;
            let chunk = newstream.next().await;
            *stream = Some(Stream::Local(newstream));
            chunk
        }
    };
    if chunk.is_none() {
        *stream = None;
    }
    drop(stream);
    if let Some(chunk) = &chunk {
        let user: user::Model = user::Entity::find_by_id(model.user_id)
            .one(db)
            .await?
            .expect("At least 1 message");
        let message: message::Model = message::Entity::find()
            .filter(message::Column::ConversationId.eq(conversationid))
            .order_by_desc(message::Column::CreatedAt)
            .one(db)
            .await?
            .expect("At least 1 message");
        if message.user_id == user.id {
            let content = message.content.clone();
            let mut message: message::ActiveModel = message.into();
            message.content = Set(format!("{}{}", content, chunk));
            message.update(db).await?;
        } else {
            let now = Utc::now();
            let message = message::ActiveModel {
                conversation_id: Set(conversationid),
                user_id: Set(user.id),
                content: Set(chunk.to_string()),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
                ..Default::default()
            };
            let _ = message.insert(db).await?;
        }
    }
    Ok(chunk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_chunk_error() {
        let error = InnerError::InvalidToken;
        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            "\"Authorization header is correct, but the token seems invalid\""
        );
        let error = r#"{"error": "Authorization header is correct, but the token seems invalid"}"#;
        let error: SseError = serde_json::from_str(&error).unwrap();
        if let SseError {
            error: InnerError::InvalidToken,
        } = error
        {
            // OK
        } else {
            panic!("Invalid deserialization of chunk error {error:?}");
        }
    }
}
