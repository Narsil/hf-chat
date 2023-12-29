use candle::Device;
use futures_util::StreamExt;
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreErrorResponseType, CoreProviderMetadata,
};
use openidconnect::reqwest::async_http_client;
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RequestTokenError, Scope, StandardErrorResponse,
};
use openidconnect::{OAuth2TokenResponse, TokenResponse};
use reqwest::header::AUTHORIZATION;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Database, DatabaseConnection, EntityTrait, ModelTrait,
    QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
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
use entities::user::{self, Model as User};
use tracing::{debug, error, info};

const TARGET: &str = env!("TARGET");

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
    Lock(#[from] tokio::sync::TryLockError),

    #[error(transparent)]
    Tokenizer(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Model {0} was not found")]
    ModelNotFound(String),

    #[error("Url error {0}")]
    OpenIdUrl(#[from] openidconnect::url::ParseError),

    #[error("Openid error {0}")]
    OpenId(#[from] OpenidError),
}

#[derive(Debug, thiserror::Error)]
pub enum OpenidError {
    #[error("Url error {0}")]
    Url(#[from] openidconnect::url::ParseError),

    #[error("Discover error {0}")]
    Discovery(#[from] openidconnect::DiscoveryError<openidconnect::reqwest::Error<reqwest::Error>>),

    #[error("Signing error {0}")]
    Signing(#[from] openidconnect::SigningError),

    #[error("Request token error {0}")]
    RequestTokenError(
        #[from]
        RequestTokenError<
            openidconnect::reqwest::Error<reqwest::Error>,
            StandardErrorResponse<CoreErrorResponseType>,
        >,
    ),

    #[error("Claims verification error {0}")]
    Claims(#[from] openidconnect::ClaimsVerificationError),

    #[error("Invalid token")]
    InvalidToken,

    #[error("Invalid csrf token")]
    InvalidCsrf,

    #[error("Missing token")]
    MissingToken,

    #[error("Unset validators")]
    UnsetValidators,
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
    cache: Cache,
    device: Device,
    openid: Mutex<Option<Openid>>,
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
    user: Option<User>,
}

#[tauri::command]
async fn settings(state: tauri::State<'_, State>, settings: Settings) -> Result<(), Error> {
    // Rename for naming sanity
    let db = &state.db;
    let model_id = settings.active_model;
    tracing::debug!("Inserting settings {model_id}");
    let model: Option<model::Model> = model::Entity::find()
        .filter(model::Column::Id.contains(model_id.clone()))
        .order_by_asc(model::Column::Name)
        .one(db)
        .await?;
    let model = model.ok_or(Error::ModelNotFound(model_id))?;
    tracing::debug!("Found in DB");
    let mut settings: settings::ActiveModel =
        settings::Entity::find().one(db).await?.unwrap().into();
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

    settings.update(db).await.ok();
    Ok(())
}

struct Openid {
    csrf_token: CsrfToken,
    nonce: Nonce,
    pkce_verifier: PkceCodeVerifier,
}

#[tauri::command]
async fn login(state: tauri::State<'_, State>) -> Result<String, Error> {
    let provider_metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new("https://huggingface.co".to_string())?,
        async_http_client,
    )
    .await
    .map_err(OpenidError::from)?;

    // Create an OpenID Connect client by specifying the client ID, client secret, authorization URL
    // and token URL.
    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new("64d7dfec-160a-41f0-921f-ab071cf4f16f".to_string()),
        None,
    )
    // Set the URL the user will be redirected to after the authorization process.
    .set_redirect_uri(RedirectUrl::new(
        "http://localhost:5173/login/callback".to_string(),
    )?);

    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        // Set the desired scopes.
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("inference-api".to_string()))
        // Set the PKCE code challenge.
        .set_pkce_challenge(pkce_challenge)
        .url();
    let mut guard = state.openid.try_lock()?;
    *guard = Some(Openid {
        csrf_token,
        nonce,
        pkce_verifier,
    });
    info!("Authentication url {auth_url}");
    Ok(auth_url.to_string())
}

#[tauri::command]
async fn logout(state: tauri::State<'_, State>) -> Result<(), Error> {
    let token_path = state.cache.token_path();
    if token_path.exists() {
        let token = std::fs::read_to_string(&token_path)?;
        if token.starts_with("hf_oauth") {
            std::fs::remove_file(token_path)?;
        }
    }
    let db = &state.db;
    let user = user::Entity::find().one(db).await.unwrap().unwrap();
    user.delete(db).await?;
    // TODO fix this find last_insert_model
    Ok(())
}

#[tauri::command]
async fn login_callback(
    app_state: tauri::State<'_, State>,
    code: String,
    state: String,
) -> Result<(), Error> {
    info!("Login callback {code} {state}");

    let mut openid = app_state.openid.try_lock()?;
    let Openid {
        csrf_token,
        nonce,
        pkce_verifier,
    } = openid.take().ok_or(OpenidError::UnsetValidators)?;
    let provider_metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new("https://huggingface.co".to_string())?,
        async_http_client,
    )
    .await
    .map_err(OpenidError::from)?;

    // Create an OpenID Connect client by specifying the client ID, client secret, authorization URL
    // and token URL.
    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new("64d7dfec-160a-41f0-921f-ab071cf4f16f".to_string()),
        None,
    )
    // Set the URL the user will be redirected to after the authorization process.
    .set_redirect_uri(RedirectUrl::new(
        "http://localhost:5173/login/callback".to_string(),
    )?);

    // Once the user has been redirected to the redirect URL, you'll have access to the
    // authorization code. For security reasons, your code should verify that the `state`
    // parameter returned by the server matches `csrf_state`.
    if csrf_token.secret() != &state {
        return Err(OpenidError::InvalidCsrf.into());
    }

    // Now you can exchange it for an access token and ID token.
    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        // Set the PKCE code verifier.
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .map_err(OpenidError::from)?;

    // Extract the ID token claims after verifying its authenticity and nonce.
    let id_token = token_response
        .id_token()
        .ok_or_else(|| OpenidError::MissingToken)?;
    let claims = id_token
        .claims(&client.id_token_verifier(), &nonce)
        .map_err(OpenidError::from)?;

    // Verify the access token hash to ensure that the access token hasn't been substituted for
    // another user's.
    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            &id_token.signing_alg().map_err(OpenidError::from)?,
        )
        .map_err(OpenidError::from)?;
        if actual_access_token_hash != *expected_access_token_hash {
            return Err(OpenidError::InvalidToken.into());
        }
    }

    // The authenticated user's identity is now available. See the IdTokenClaims struct for a
    // complete listing of the available claims.
    let token_path = app_state.cache.token_path();
    let token = token_response.access_token().secret();
    if !token_path.exists() {
        if let Ok(mut file) = std::fs::File::create(token_path) {
            file.write_all(token.as_bytes())?;
        }
    }
    let name = claims
        .name()
        .and_then(|name| name.get(None))
        .map(|name| name.as_str())
        .unwrap_or("<not provided>");
    let db = &app_state.db;
    let new_user = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        username: Set(name.to_string()),
        email: Set("".to_string()),
    };

    new_user.insert(db).await.ok();
    // TODO fix this find last_insert_model
    // let user = user::Entity::find().one(db).await.unwrap().unwrap();
    Ok(())
}

#[tauri::command]
async fn load(state: tauri::State<'_, State>) -> Result<Load, Error> {
    let db = &state.db;
    let conversations = conversation::Entity::find().all(db).await?;
    let models = model::Entity::find().all(db).await?;
    // let active_model = "tiiuae/falcon-180B-chat".into();
    let settings = match settings::Entity::find().one(db).await? {
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

            new_settings.insert(db).await.ok();
            // TODO fix this find last_insert_model
            settings::Entity::find().one(db).await.unwrap().unwrap()
        }
    };
    let user = user::Entity::find().one(db).await?;
    info!("Found user {user:?}");
    let token = state.cache.token().clone();
    let load = Load {
        conversations,
        models,
        old_models: vec![],
        settings,
        messages_before_login: 0,
        requires_login: false,
        token,
        user,
    };
    Ok(load)
}

#[allow(unused_variables)]
fn cache(path: &Path) -> Cache {
    #[cfg(not(mobile))]
    let cache = Cache::default();
    #[cfg(mobile)]
    let cache = {
        std::fs::create_dir_all(path).expect("Could not create dir");
        let cache = Cache::new(path.to_path_buf());
        let token_path = cache.token_path();
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
    let db = &state.db;
    // Rename for naming sanity
    let model_id = model;
    let model: Option<model::Model> = model::Entity::find()
        .filter(model::Column::Id.contains(model_id.clone()))
        .order_by_asc(model::Column::Name)
        .one(db)
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

    conversation.insert(db).await.ok();
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
    let db = &state.db;
    let conversation: Option<Conversation> = conversation::Entity::find()
        .filter(conversation::Column::Id.eq(id))
        .one(db)
        .await?;
    let conversation = conversation.unwrap();
    // Then, find all related fruits of this cake
    let messages: Vec<Message> = conversation.find_related(message::Entity).all(db).await?;
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
    state: tauri::State<'_, State>,
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
    info!("Sending token {token:?}");
    if let Some(token) = token {
        request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }

    let db = state.db.clone();
    tokio::task::spawn(async move {
        let mut stream = request.send().await?.bytes_stream();

        while let Some(item) = stream.next().await {
            info!("Received items {item:?}");
            let item = item?;
            let chunk = &item["data:".len()..];
            let generation: Generation = serde_json::from_slice(chunk)?;
            // info!("Chunk: {:?}", generation);
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
                message.insert(&db).await.ok();
            }

            app.emit("text-generation", generation)?;
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
    info!("Generate {url} on device {:?}", state.device);
    let query = Query {
        inputs,
        parameters,
        stream: true,
    };
    let (newtx, mut rx) = tokio::sync::oneshot::channel();
    let cache = state.cache.clone();
    let device = state.device.clone();
    tokio::task::spawn_blocking(move || {
        if model == "karpathy/tinyllamas" {
            let mut pipeline = crate::local::llama_c::load_local(query, device, &cache)?;
            for generation in pipeline.iter() {
                let generation = generation?;
                app.emit("text-generation", generation)?;
                if let Ok(_) = rx.try_recv() {
                    break;
                }
            }
        } else if model == "microsoft/phi-1_5" {
            let device = Device::Cpu;
            let mut pipeline = crate::local::phi::load_local(query, device, &cache)?;
            info!("Loaded pipeline");
            for generation in pipeline.iter() {
                let generation = generation?;
                info!("Emitting {generation:?}");
                app.emit("text-generation", generation)?;
                if let Ok(_) = rx.try_recv() {
                    break;
                }
            }
        } else {
            let mut pipeline = crate::local::llama::load_local(query, device, &cache)?;
            for generation in pipeline.iter() {
                let generation = generation?;
                app.emit("text-generation", generation)?;
                if let Ok(_) = rx.try_recv() {
                    break;
                }
            }
        };
        Ok::<(), Error>(())
    });
    let mut tx = state.tx.try_lock()?;
    let tmptx = (*tx).take();
    if let Some(tx) = tmptx {
        if let Err(_) = tx.send(()) {
            // error!("Could not send stop signal");
        }
    }
    *tx = Some(newtx);
    Ok(())
}

#[tauri::command]
async fn stop(state: tauri::State<'_, State>) -> Result<(), Error> {
    tracing::info!("STOP");
    let mut tx = state.tx.try_lock()?;
    let tmptx = (*tx).take();
    if let Some(tx) = tmptx {
        if let Err(_) = tx.send(()) {
            error!("Could not send stop signal");
        }
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
    let db = &state.db;
    let message = message::ActiveModel {
        conversation_id: Set(conversation_id),
        id: Set(Uuid::new_v4()),
        from: Set("user".into()),
        content: Set(inputs.clone()),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };

    message.insert(db).await.ok();
    match &model[..] {
        "tiiuae/falcon-180B-chat" => {
            info!("New falcon message");
            let inputs = build_falcon_prompt(inputs);
            query_api(
                app,
                state.clone(),
                model,
                conversation_id,
                inputs,
                parameters,
                state.cache.token().as_ref(),
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

#[derive(Default)]
pub struct AppBuilder {}

async fn init_db(cache: &Cache) -> Result<DatabaseConnection, Error> {
    let mut path = cache.path().clone();
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

    pub fn run(self) {
        tauri::Builder::default()
            .plugin(tauri_plugin_log::Builder::default().build())
            .invoke_handler(tauri::generate_handler![
                load,
                conversation,
                load_conversation,
                generate,
                stop,
                settings,
                login,
                login_callback,
                logout,
            ])
            .setup(move |app| {
                info!("Start the run");
                info!(
                    "avx: {}, neon: {}, simd128: {}, f16c: {}",
                    candle::utils::with_avx(),
                    candle::utils::with_neon(),
                    candle::utils::with_simd128(),
                    candle::utils::with_f16c()
                );
                let path = app.path().local_data_dir().expect("Have a local data dir");
                let cache = cache(&path);
                tracing::info!("Start the db");
                let db = tauri::async_runtime::block_on(async {
                    init_db(&cache).await.expect("Failed to create db")
                });
                tracing::info!("get the device");
                let device = if candle::utils::cuda_is_available() {
                    Device::new_cuda(0)?
                // Simulator doesn't support MPS (Metal Performance Shader).
                } else if candle::utils::metal_is_available() && TARGET != "aarch64-apple-ios-sim" {
                    Device::new_metal(0)?
                } else {
                    Device::Cpu
                };
                app.manage(State {
                    db,
                    cache,
                    device,
                    openid: Mutex::new(None),
                    tx: Mutex::new(None),
                });
                // if let Some(setup) = setup {
                //     (setup)(app)?;
                // }
                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}
