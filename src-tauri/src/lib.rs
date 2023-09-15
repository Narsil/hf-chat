use sea_orm::{ConnectionTrait, Database, DbBackend, DbErr, Statement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::App;
use url::Url;

#[cfg(not(feature = "mobile"))]
use hf_hub::Cache;

mod entities;
pub mod migrations;
use entities::conversation::Model as Conversation;
use entities::model::{Model, Parameters, PromptExample, Prompts};

#[cfg(mobile)]
mod mobile;
#[cfg(mobile)]
pub use mobile::*;

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
    init_db().await;
    let conversations = vec![];
    let models = vec![Model {
        internal_id: 0,
        id: "tiiuae/falcon-180B-chat".into(),
        name: "tiiuae/falcon-180B-chat".into(),
        website_url: "https://api-inference.huggingface.co/models/tiiuae/falcon-180B-chat".into(),
        dataset_name: "OpenAssistant/oasst1".into(),
        display_name: "tiiuae/falcon-180B-chat".into(),
        description: "A good alternative to ChatGPT".into(),
        prompt_examples: Prompts{prompts: vec![PromptExample{ title: "Write an email from bullet list".into(), prompt: "As a restaurant owner, write a professional email to the supplier to get these products every week: \n\n- Wine (x10)\n- Eggs (x24)\n- Bread (x12)".into() }, ]},
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
    let token = cache().token();
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

fn cache() -> Cache {
    #[cfg(not(feature = "mobile"))]
    let cache = Cache::default();
    #[cfg(feature = "mobile")]
    let cache = {
        let path = std::path::Path::new("/data/data/co/huggingface/databases");
        let cache = Cache::new(path);
    };
    cache
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

pub type SetupHook = Box<dyn FnOnce(&mut App) -> Result<(), Box<dyn std::error::Error>> + Send>;

#[derive(Default)]
pub struct AppBuilder {
    setup: Option<SetupHook>,
}
async fn init_db() {
    let mut path = cache().path().clone();
    path.push("chat");
    path.push("db.sqlite");
    if !path.exists() {
        let mut dir = path.clone();
        dir.pop();
        std::fs::create_dir_all(dir).ok();
        let mut file = std::fs::File::create(path.clone()).unwrap();
    } else {
    }

    let db = Database::connect(format!("sqlite:{}", path.to_str().unwrap()))
        .await
        .unwrap();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Response {
    ok: bool,
    content: String,
}

#[tauri::command]
async fn fetch(url: String, opts: Option<serde_json::Value>) -> Result<Response, String> {
    let url = Url::parse(&url).map_err(|e| e.to_string())?;
    let content = match url.path() {
        // "/__data.json" => {
        //     let content = load().await?;
        //     serde_json::to_string(&content).map_err(|e| e.to_string())?
        // }
        path => {
            println!("Unkown path {path}");
            "Hello".into()
        }
    };
    println!("Fetch {url} {opts:?}");
    Ok(Response { ok: true, content })
}

impl AppBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn setup<F>(mut self, setup: F) -> Self
    where
        F: FnOnce(&mut App) -> Result<(), Box<dyn std::error::Error>> + Send + 'static,
    {
        self.setup.replace(Box::new(setup));
        self
    }

    pub fn run(self) {
        let setup = self.setup;
        tauri::Builder::default()
            .invoke_handler(tauri::generate_handler![
                load,
                conversation,
                load_conversation,
                fetch,
            ])
            .setup(move |app| {
                if let Some(setup) = setup {
                    (setup)(app)?;
                }
                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}
