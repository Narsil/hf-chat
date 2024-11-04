use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub profile: String,
    // pub is_me: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Message {
    pub content: String,
    pub author: User,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Conversation {
    pub messages: Vec<Message>,
}
