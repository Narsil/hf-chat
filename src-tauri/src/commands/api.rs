use crate::entities::{conversation, message, model};
use crate::State;
use ::reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Response,
};
use core::str;
use log::info;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

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
    role: Role,
    content: String,
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

pub struct Stream {
    res: Response,
    leftover: Vec<u8>,
}

async fn query(url: String, messages: Vec<Message>) -> Result<Stream, Error> {
    let client = ::reqwest::Client::new();
    let cache = hf_hub::Cache::default();
    let token = cache.token().expect("Expected token");
    let model = "tgi".to_string();

    let stream = true;
    let max_tokens = 200;
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
    let res = res.error_for_status()?;
    return Ok(Stream {
        res,
        leftover: vec![],
    });
}

impl Stream {
    async fn next(&mut self) -> Result<Option<String>, Error> {
        if let Some(chunk) = self.res.chunk().await? {
            let mut content = String::new();

            for (i, mut subchunk) in chunk.split(|&c| c == b'\n').enumerate() {
                if i == 0 && !self.leftover.is_empty() {
                    self.leftover.extend(subchunk);
                    subchunk = &self.leftover[..];
                }
                if subchunk.starts_with(b"data: ") {
                    if subchunk == b"data: [DONE]" {
                        continue;
                    }
                    if let Ok(parsed) =
                        serde_json::from_slice::<Chunk>(&subchunk[b"data: ".len()..])
                    {
                        content.push_str(&parsed.choices[0].delta.content);
                    } else {
                        let owned = subchunk.to_owned();
                        self.leftover.extend(owned);
                    }
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
    let mut stream = state.stream.lock().await;
    let chunk = if let Some(ref mut stream) = &mut *stream {
        stream.next().await?
    } else {
        let db = &state.db;
        let (_conversation, model): (conversation::Model, Option<model::Model>) =
            conversation::Entity::find_by_id(conversationid)
                .find_also_related(model::Entity)
                .one(db)
                .await?
                .ok_or(Error::MissingConversation(conversationid))?;
        let messages: Vec<message::Model> = message::Entity::find()
            .filter(message::Column::ConversationId.eq(conversationid))
            .all(db)
            .await?;

        let messages = Message::from_db(messages);
        info!("Sending messages {messages:?}");
        let url = model.expect("Model").endpoint;
        let mut newstream = query(url, messages).await?;
        let chunk = newstream.next().await.expect("chunk");
        *stream = Some(newstream);
        chunk
    };
    // info!("Got chunk {chunk:?}");
    if chunk.is_none() {
        *stream = None;
    }
    Ok(chunk)
}
