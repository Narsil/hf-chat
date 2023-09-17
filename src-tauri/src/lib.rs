use futures_util::StreamExt;
use reqwest::header::AUTHORIZATION;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::App;
use tauri::Manager;

use hf_hub::Cache;

mod entities;
// mod local;
pub mod migrations;
use entities::conversation::Model as Conversation;
use entities::model::{Model, Parameters, PromptExample, Prompts};
// use local::load;
use tracing::info;

#[cfg(mobile)]
mod mobile;
#[cfg(mobile)]
pub use mobile::*;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    DbErr(#[from] sea_orm::DbErr),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Tauri(#[from] tauri::Error),
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

// impl std::fmt::Display for Error {
//     fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
//         write!(fmt, "{self}")
//     }
// }

struct State {
    db: DatabaseConnection,
    token: Option<String>,
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
    let falcon = Model {
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
            return_full_text: false,
        }, preprompt: "".into()};
    let llama = Model {
        internal_id: 0,
        id: "meta-llama/Llama-2-7b-chat-hf".into(),
        name: "meta-llama/Llama-2-7b-chat-hf".into(),
        website_url: "https://api-inference.huggingface.co/models/meta-llama/Llama-2-7b-chat-hf".into(),
        dataset_name: "".into(),
        display_name: "meta-llama/Llama-2-7b-chat-hf".into(),
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
            return_full_text: false,
        },
        preprompt: "".into(),
    };
    let models = vec![llama, falcon];
    let active_model = "meta-llama/Llama-2-7b-chat-hf".into();
    // let active_model = "tiiuae/falcon-180B-chat".into();
    let settings = Settings {
        share_conversations_with_model_authors: true,
        ethics_model_accepted_at: None,
        active_model,
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
    #[cfg(not(mobile))]
    let cache = Cache::default();
    #[cfg(mobile)]
    let cache = {
        let path = std::path::Path::new("/data/data/co/huggingface/databases");
        let cache = Cache::new(path.to_path_buf());
        cache
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
        model: "meta-llama/Llama-2-7b-chat-hf".into(),
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Query {
    inputs: String,
    parameters: Parameters,
    stream: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Token {
    id: usize,
    text: String,
    logprob: f32,
    special: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Generation {
    token: Token,
    generated_text: Option<String>,
    details: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Options {
    id: u32,
    response_id: String,
    is_retry: bool,
    use_cache: bool,
    web_search_id: String,
}

fn build_falcon_prompt(inputs: String) -> String {
    format!("User:{inputs}\nFalcon:")
}

fn build_llama_prompt(inputs: String) -> String {
    let system_prompt = r#"You are a helpful, respectful and honest assistant. Always answer as helpfully as possible, while being safe.  Your answers should not include any harmful, unethical, racist, sexist, toxic, dangerous, or illegal content. Please ensure that your responses are socially unbiased and positive in nature.

If a question does not make any sense, or is not factually coherent, explain why instead of answering something not correct. If you don't know the answer to a question, please don't share false information.
"#;
    format!(
        r#"<s>[INST] <<SYS>>
{system_prompt}
<</SYS>>

{inputs} [/INST]"#
    )
}

fn query_api(
    app: tauri::AppHandle,
    model: String,
    inputs: String,
    parameters: Parameters,
    token: Option<&String>,
) -> Result<(), Error> {
    let url = format!("https://api-inference.huggingface.co/models/{model}");
    info!("Generate {url}");
    let query = Query {
        inputs,
        parameters,
        stream: true,
    };
    let client = reqwest::Client::new();
    let mut request = client.post(url).json(&query).header("x-use-cache", "0");
    if let Some(token) = token {
        request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }

    tokio::task::spawn(async move {
        let mut stream = request.send().await?.bytes_stream();

        while let Some(item) = stream.next().await {
            let item = item?;
            let chunk = &item["data:".len()..];
            let generation: Generation = serde_json::from_slice(chunk)?;
            // println!("Chunk: {:?}", generation);
            // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            app.emit_all("text-generation", generation)?;
        }
        Ok::<(), Error>(())
    });
    Ok(())
}

fn query_local(
    app: tauri::AppHandle,
    model: String,
    inputs: String,
    parameters: Parameters,
    token: Option<&String>,
) -> Result<(), Error> {
    let url = format!("https://api-inference.huggingface.co/models/{model}");
    info!("Generate {url}");
    let query = Query {
        inputs,
        parameters,
        stream: true,
    };
    tokio::task::spawn(async move {
        // let pipeline = load_local(query);
        // for generation in pipeline.iter() {
            for i in 0..query.parameters.max_new_tokens {
                let generated_text = if i == 9 {
                    Some("finished !".into())
                } else {
                    None
                };
                let generation = Generation {
                    token: Token {
                        id: 0,
                        logprob: 0.0,
                        text: format!("{i} "),
                        special: false,
                    },
                    generated_text,
                    details: None,
                };
                // println!("Chunk: {:?}", generation);
                // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            app.emit_all("text-generation", generation)?;
        }
        Ok::<(), Error>(())
    });
    Ok(())
}

#[tauri::command]
async fn generate(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
    model: String,
    inputs: String,
    parameters: Parameters,
    // options: Options,
) -> Result<(), Error> {
    tracing::debug!("Generating for {model}");
    match &model[..] {
        "tiiuae/falcon-180B-chat" => {
            let inputs = build_falcon_prompt(inputs);
            query_api(app, model, inputs, parameters, state.token.as_ref())
        }
        "meta-llama/Llama-2-7b-chat-hf" => {
            let inputs = build_llama_prompt(inputs);
            // query_api(app, model, inputs, parameters, state.token.as_ref())
            query_local(app, model, inputs, parameters, state.token.as_ref())
        }
        model => todo!("Need to implement proper template {model}"),
    }
}

pub type SetupHook = Box<dyn FnOnce(&mut App) -> Result<(), Box<dyn std::error::Error>> + Send>;

#[derive(Default)]
pub struct AppBuilder {
    setup: Option<SetupHook>,
}
async fn init_db() -> Result<DatabaseConnection, Error> {
    let mut path = cache().path().clone();
    path.push("chat");
    path.push("db.sqlite");
    if !path.exists() {
        let mut dir = path.clone();
        dir.pop();
        std::fs::create_dir_all(dir).ok();
        std::fs::File::create(path.clone())?;
    } else {
    }

    let filename = format!("sqlite:{}", path.display());
    Ok(Database::connect(filename).await?)
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

    pub async fn run(self) {
        let setup = self.setup;
        tracing_subscriber::fmt::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
        let db = init_db().await.unwrap();
        let token = cache().token();

        tauri::Builder::default()
            .manage(State { db, token })
            .invoke_handler(tauri::generate_handler![
                load,
                conversation,
                load_conversation,
                generate,
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
