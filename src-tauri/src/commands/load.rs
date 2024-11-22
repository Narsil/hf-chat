use crate::entities::conversation;
use crate::entities::model;
use crate::entities::user;
use crate::State;
use log::info;
use sea_orm::{EntityTrait, FromQueryResult, JoinType, QueryOrder, QuerySelect, RelationTrait};
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
    Api(#[from] hf_hub::api::tokio::ApiError),

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
    // OpenId(#[from] openidconnect::OpenidError),
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

#[derive(Debug, Clone, Deserialize, Serialize, FromQueryResult)]
pub struct ConversationList {
    pub id: u32,
    pub title: String,
    pub profile: String,
    pub user_id: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Load {
    conversations: Vec<ConversationList>,
    user: Option<user::Model>,
    users: Vec<user::Model>,
}

#[tauri::command]
pub async fn load(state: tauri::State<'_, State>) -> Result<Load, Error> {
    let db = &state.db;
    let conversations = conversation::Entity::find()
        .order_by_desc(conversation::Column::CreatedAt)
        .select_only()
        .column(conversation::Column::Title)
        .column(conversation::Column::Id)
        .column_as(user::Column::Profile, "profile")
        .column_as(user::Column::Id, "user_id")
        .join(JoinType::InnerJoin, conversation::Relation::Model.def())
        .join(JoinType::InnerJoin, model::Relation::User.def())
        .into_model::<ConversationList>()
        .all(db)
        .await
        .expect("query");
    let users = user::Entity::find().all(db).await?;
    let user = users.first().cloned();
    info!("Found user {:?}", user.as_ref().map(|u| &u.name));
    let load = Load {
        conversations,
        user,
        users,
    };
    Ok(load)
}
