use futures_util::StreamExt;
use reqwest::header::AUTHORIZATION;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Database, DatabaseConnection, EntityTrait, ModelTrait,
    QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::App;
use tauri::Manager;
use tokio::sync::Mutex;
use uuid::Uuid;

use hf_hub::Cache;

mod entities;
mod local;
pub mod migrations;
use entities::conversation::{self, Model as Conversation};
use entities::message::{self, Model as Message};
use entities::model::{self, Model, Parameters};
use entities::settings::{self, CustomPrompts, Model as Settings};
use local::llama::load_local;
use tracing::{debug, info};

#[cfg(mobile)]
mod mobile;
#[cfg(mobile)]
pub use mobile::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
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

    #[error(transparent)]
    Api(#[from] hf_hub::api::sync::ApiError),

    #[error(transparent)]
    Candle(#[from] candle::Error),

    #[error(transparent)]
    Tokenizer(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Model {0} was not found")]
    ModelNotFound(String),
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
    tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
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
async fn settings(state: tauri::State<'_, State>, settings: Settings) -> Result<(), Error> {
    // Rename for naming sanity
    let model_id = settings.active_model;
    tracing::debug!("Inserting settings {model_id}");
    let model: Option<model::Model> = model::Entity::find()
        .filter(model::Column::Id.contains(model_id.clone()))
        .order_by_asc(model::Column::Name)
        .one(&state.db)
        .await?;
    let model = model.ok_or(Error::ModelNotFound(model_id))?;
    tracing::debug!("Found in DB");
    let mut settings: settings::ActiveModel = settings::Entity::find()
        .one(&state.db)
        .await?
        .unwrap()
        .into();
    settings.active_model = Set(model.id);
    // let settings = settings::ActiveModel {
    //     id: Set(Uuid::new_v4()),
    //     active_model: Set(active_model.into()),
    //     share_conversations_with_model_authors: Set(true),
    //     ethics_model_accepted_at: Set(None),
    //     search_enabled: Set(false),
    //     custom_prompts: Set(CustomPrompts {
    //         prompts: HashMap::new(),
    //     }),
    // };

    settings.update(&state.db).await.ok();
    Ok(())
}

#[tauri::command]
async fn load(state: tauri::State<'_, State>) -> Result<Load, Error> {
    let conversations = conversation::Entity::find().all(&state.db).await?;
    let models = model::Entity::find().all(&state.db).await?;
    // let active_model = "tiiuae/falcon-180B-chat".into();
    let settings = match settings::Entity::find().one(&state.db).await? {
        Some(settings) => settings,
        None => {
            let active_model = &models[0].id;
            let new_settings = settings::ActiveModel {
                id: Set(Uuid::new_v4()),
                active_model: Set(active_model.into()),
                share_conversations_with_model_authors: Set(true),
                ethics_model_accepted_at: Set(None),
                search_enabled: Set(false),
                custom_prompts: Set(CustomPrompts {
                    prompts: HashMap::new(),
                }),
            };

            new_settings.insert(&state.db).await.ok();
            // TODO fix this find last_insert_model
            settings::Entity::find()
                .one(&state.db)
                .await
                .unwrap()
                .unwrap()
        }
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
        let path = std::path::Path::new("/data/data/co.huggingface.chat/cache/");
        std::fs::create_dir_all(path).expect("Could not create dir");
        let cache = Cache::new(path.to_path_buf());
        let token_path = cache.token_path();
        if !token_path.exists() {
            use std::io::Write;
            let mut file = std::fs::File::create(token_path).unwrap();
            file.write(b"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")
                .unwrap();
        }
        cache
    };
    cache
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConversationResponse {
    conversation_id: Uuid,
}

#[tauri::command]
async fn conversation(
    state: tauri::State<'_, State>,
    model: String,
) -> Result<ConversationResponse, Error> {
    // Rename for naming sanity
    let model_id = model;
    let model: Option<model::Model> = model::Entity::find()
        .filter(model::Column::Id.contains(model_id.clone()))
        .order_by_asc(model::Column::Name)
        .one(&state.db)
        .await?;
    let model = model.ok_or(Error::ModelNotFound(model_id))?;
    let id = Uuid::new_v4();
    let conversation = conversation::ActiveModel {
        model_id: Set(Some(model.id.clone())),
        id: Set(id),
        title: Set("Conversation".into()),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };

    conversation.insert(&state.db).await.ok();
    Ok(ConversationResponse {
        conversation_id: id,
    })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ConversationView {
    messages: Vec<Message>,
    title: String,
    model: String,
    searches: Vec<Message>,
}

#[tauri::command]
async fn load_conversation(
    state: tauri::State<'_, State>,
    id: Uuid,
) -> Result<ConversationView, Error> {
    let conversation: Option<Conversation> = conversation::Entity::find()
        .filter(conversation::Column::Id.eq(id))
        .one(&state.db)
        .await?;
    let conversation = conversation.unwrap();
    // Then, find all related fruits of this cake
    let messages: Vec<Message> = conversation
        .find_related(message::Entity)
        .all(&state.db)
        .await?;
    Ok(ConversationView {
        model: conversation.model_id.clone().unwrap(),
        // model: "codellama/CodeLlama-7b-hf".into(),
        title: conversation.title.clone(),
        messages,
        // messages: vec![Message {
        //     content: "User: Hello".into(),
        //     from: "user".into(),
        //     id: "xxx".into(),
        // }],
        searches: vec![],
    })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Query {
    inputs: String,
    parameters: Parameters,
    stream: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Token {
    id: usize,
    text: String,
    logprob: f32,
    special: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Generation {
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
        r#"[INST] <<SYS>>
{system_prompt}
<</SYS>>

{inputs} [/INST]"#
    )
}

fn query_api(
    app: tauri::AppHandle,
    model: String,
    conversation_id: Uuid,
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
            if let Some(generated_text) = &generation.generated_text {
                let message = message::ActiveModel {
                    conversation_id: Set(conversation_id),
                    id: Set(Uuid::new_v4()),
                    from: Set("assistant".into()),
                    content: Set(generated_text.clone()),
                    created_at: Set(chrono::Utc::now()),
                    updated_at: Set(chrono::Utc::now()),
                };
                let db = init_db().await?;
                message.insert(&db).await.ok();
            }

            app.emit_all("text-generation", generation)?;
        }
        Ok::<(), Error>(())
    });
    Ok(())
}

async fn query_local(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
    model: String,
    inputs: String,
    parameters: Parameters,
) -> Result<(), Error> {
    let url = format!("https://api-inference.huggingface.co/models/{model}");
    info!("Generate {url}");
    let query = Query {
        inputs,
        parameters,
        stream: true,
    };
    let (newtx, mut rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || {
        if model == "karpathy/tinyllamas" {
            let mut pipeline = crate::local::llama_c::load_local(query)?;
            for generation in pipeline.iter() {
                let generation = generation?;
                app.emit_all("text-generation", generation)?;
                if let Ok(_) = rx.try_recv() {
                    break;
                }
            }
        } else if model == "microsoft/phi-1_5" {
            let mut pipeline = crate::local::phi::load_local(query)?;
            for generation in pipeline.iter() {
                let generation = generation?;
                app.emit_all("text-generation", generation)?;
                if let Ok(_) = rx.try_recv() {
                    break;
                }
            }
        } else {
            let mut pipeline = load_local(query)?;
            for generation in pipeline.iter() {
                let generation = generation?;
                app.emit_all("text-generation", generation)?;
                if let Ok(_) = rx.try_recv() {
                    break;
                }
            }
        };
        Ok::<(), Error>(())
    });
    let mut tx = state.tx.lock().await;
    let tmptx = (*tx).take();
    if let Some(tx) = tmptx {
        tx.send(()).unwrap();
    }
    *tx = Some(newtx);
    Ok(())
}

#[tauri::command]
async fn stop(state: tauri::State<'_, State>) -> Result<(), Error> {
    tracing::info!("STOP");
    let mut tx = state.tx.lock().await;
    let tmptx = (*tx).take();
    if let Some(tx) = tmptx {
        tx.send(()).unwrap();
    }
    Ok(())

    // if let Some(tx) = *tx {
    //     Ok(tx.send(()).unwrap())
    // } else {
    //     Ok(())
    // }
}

#[tauri::command]
async fn generate(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
    model: String,
    conversation_id: Uuid,
    inputs: String,
    parameters: Parameters,
    // options: Options,
) -> Result<(), Error> {
    tracing::debug!("Generating for {model}");
    let message = message::ActiveModel {
        conversation_id: Set(conversation_id),
        id: Set(Uuid::new_v4()),
        from: Set("user".into()),
        content: Set(inputs.clone()),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };

    message.insert(&state.db).await.ok();
    match &model[..] {
        "tiiuae/falcon-180B-chat" => {
            let inputs = build_falcon_prompt(inputs);
            query_api(
                app,
                model,
                conversation_id,
                inputs,
                parameters,
                state.token.as_ref(),
            )
        }
        "meta-llama/Llama-2-7b-chat-hf" => {
            let inputs = build_llama_prompt(inputs);
            // query_api(app, model, inputs, parameters, state.token.as_ref())
            query_local(app, state, model, inputs, parameters).await
        }
        "karpathy/tinyllamas" => query_local(app, state, model, inputs, parameters).await,
        "microsoft/phi-1_5" => query_local(app, state, model, inputs, parameters).await,
        model => Err(Error::ModelNotFound(model.to_string())),
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
        debug!("Attempting to create dir {}", dir.display());
        std::fs::create_dir_all(dir).expect("Could not create dir");
        std::fs::File::create(path.clone()).expect("Create file");
    };

    use sea_orm_migration::MigratorTrait;
    let filename = format!("sqlite:{}", path.display());
    let db = Database::connect(filename).await?;
    migrations::Migrator::up(&db, None).await?;
    info!("Ran migrations");
    Ok(db)
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
        println!("Start the run");
        tracing_subscriber::fmt::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
        #[cfg(mobile)]
        android_logger::init_once(
            android_logger::Config::default().with_max_level(tracing::log::LevelFilter::Trace),
        );
        println!(
            "avx: {}, neon: {}, simd128: {}, f16c: {}",
            candle::utils::with_avx(),
            candle::utils::with_neon(),
            candle::utils::with_simd128(),
            candle::utils::with_f16c()
        );
        tracing::info!("Start the db");
        let db = init_db().await.expect("Failed to create db");
        tracing::info!("get the token");
        let token = cache().token();

        tauri::Builder::default()
            .plugin(tauri_plugin_fs::init())
            .manage(State {
                db,
                token,
                tx: Mutex::new(None),
            })
            .invoke_handler(tauri::generate_handler![
                load,
                conversation,
                load_conversation,
                generate,
                stop,
                settings,
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
