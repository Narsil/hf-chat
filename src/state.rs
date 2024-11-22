use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub profile: String,
    // pub is_me: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Message {
    pub content: String,
    pub user_id: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Conversation {
    pub id: u32,
    pub title: String,
    pub profile: String,
    pub user_id: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FullConversation {
    pub id: u32,
    pub title: String,
    pub model_id: u32,
    pub messages: Vec<Message>,
}
