#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use hf_hub::Cache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// Prevents additional console window on Windows in release, DO NOT REMOVE!!

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Conversation {
    id: String,
    title: String,
    model: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    share_conversations_with_model_authors: bool,
    ethics_model_accepted_at: Option<bool>,
    active_model: String,
    search_enabled: bool,
    custom_prompts: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PromptExample {
    title: String,
    prompt: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Parameters {
    temperature: f32,
    truncate: usize,
    max_new_tokens: usize,
    stop: Vec<String>,
    top_p: f32,
    top_k: usize,
    repetition_penalty: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Model {
    id: String,
    name: String,
    website_url: String,
    dataset_name: String,
    display_name: String,
    description: String,
    prompt_examples: Vec<PromptExample>,
    parameters: Parameters,
    preprompt: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Load {
    conversations: Vec<Conversation>,
    settings: Settings,
    models: Vec<Model>,
    old_models: Vec<Model>,
    requires_login: bool,
    messages_before_login: usize,
    token: Option<String>,
}

#[tauri::command]
async fn load() -> Result<Load, String> {
    let conversations = vec![];
    let models = vec![Model {
        id: "tiiuae/falcon-180B-chat".into(),
        name: "tiiuae/falcon-180B-chat".into(),
        website_url: "https://api-inference.huggingface.co/models/tiiuae/falcon-180B-chat".into(),
        dataset_name: "OpenAssistant/oasst1".into(),
        display_name: "tiiuae/falcon-180B-chat".into(),
        description: "A good alternative to ChatGPT".into(),
        prompt_examples: vec![PromptExample{ title: "Write an email from bullet list".into(), prompt: "As a restaurant owner, write a professional email to the supplier to get these products every week: \n\n- Wine (x10)\n- Eggs (x24)\n- Bread (x12)".into() }, ],
        parameters: Parameters {
            temperature: 0.9,
            truncate: 1000,
            max_new_tokens: 20,
            stop: vec!["<|endoftext|>".into(), "Falcon:".into(), "User:".into()],
            top_p: 0.95,
            repetition_penalty: 1.2,
            top_k: 50,
        },
        preprompt: "".into(),
    }];
    let settings = Settings {
        share_conversations_with_model_authors: true,
        ethics_model_accepted_at: None,
        active_model: "tiiuae/falcon-180B-chat".into(),
        search_enabled: false,
        custom_prompts: HashMap::new(),
    };
    let cache = Cache::default();
    let token = cache.token();
    let load = Load {
        conversations,
        models,
        old_models: vec![],
        settings,
        messages_before_login: 0,
        requires_login: false,
        token,
    };
    Ok(load)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConversationResponse {
    conversation_id: String,
}

#[tauri::command]
async fn conversation(model: String) -> Result<ConversationResponse, String> {
    Ok(ConversationResponse {
        conversation_id: "000000000000".into(),
    })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Message {
    content: String,
    from: String,
    id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ConversationView {
    messages: Vec<Message>,
    title: String,
    model: String,
    searches: Vec<Message>,
}

#[tauri::command]
async fn load_conversation(id: String) -> Result<ConversationView, String> {
    Ok(ConversationView {
        model: "tiiuae/falcon-180B-chat".into(),
        // model: "codellama/CodeLlama-7b-hf".into(),
        title: "Test".into(),
        messages: vec![],
        // messages: vec![Message {
        //     content: "User: Hello".into(),
        //     from: "user".into(),
        //     id: "xxx".into(),
        // }],
        searches: vec![],
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load,
            conversation,
            load_conversation
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
