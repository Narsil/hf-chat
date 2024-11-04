use crate::commands::conversation::Conversation;
use crate::entities::conversation;
use crate::entities::message;
use crate::entities::user;
use crate::State;
use log::info;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    DbErr(#[from] sea_orm::DbErr),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    // #[error(transparent)]
    // Request(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    #[error(transparent)]
    Api(#[from] hf_hub::api::sync::ApiError),

    // #[error(transparent)]
    // Candle(#[from] candle::Error),

    // #[error(transparent)]
    // Lock(#[from] tokio::sync::TryLockError),
    #[error(transparent)]
    Tokenizer(#[from] Box<dyn std::error::Error + Send + Sync>),
    // #[error("Model {0} was not found")]
    // ModelNotFound(String),
    // #[error("Url error {0}")]
    // OpenIdUrl(#[from] openidconnect::url::ParseError),

    // #[error("Openid error {0}")]
    // OpenId(#[from] OpenidError),
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
pub struct Load {
    conversations: Vec<Conversation>,
    user: Option<user::Model>,
}

#[tauri::command]
pub async fn load(state: tauri::State<'_, State>) -> Result<Load, Error> {
    let db = &state.db;
    let conversations: Vec<(conversation::Model, Vec<message::Model>)> =
        conversation::Entity::find()
            .find_with_related(message::Entity)
            .all(db)
            .await?;
    let conversations = conversations.into_iter().map(Conversation::from).collect();
    let user = user::Entity::find().one(db).await?;
    info!("Found user {user:?}");
    let load = Load {
        conversations,
        user,
    };
    Ok(load)
}
