use crate::{entities::model, State};
use ::reqwest::{header::AUTHORIZATION, Client};
use chrono::{DateTime, TimeDelta, Utc};
use hf_hub::{
    api::tokio::{ApiBuilder, ApiError},
    Cache,
};
use log::{debug, error, info};
use sea_orm::{prelude::*, ActiveValue::Set};
use serde::{Deserialize, Serialize};
use std::io::Write;
use tokio::task::JoinSet;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No models")]
    NoModels,

    #[error("Missing token")]
    MissingToken,

    #[error("Api error {0}")]
    ApiError(#[from] ApiError),

    #[error("Reqwest error {0}")]
    ReqwestError(#[from] ::reqwest::Error),

    #[error("deserialization error {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("Io error {0}")]
    IoError(#[from] std::io::Error),

    #[error("Db error {0}")]
    DbError(#[from] sea_orm::DbErr),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[derive(Clone, Serialize)]
pub struct ModelSuggestion {
    name: String,
    full_name: String,
    profile: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Model {
    _id: String,
    model_id: String,
    downloads: usize,
    created_at: DateTimeUtc,
}

impl Model {
    fn score(&self, now: &DateTime<Utc>) -> usize {
        let on_for = now
            .signed_duration_since(self.created_at)
            .max(TimeDelta::days(14));

        let downloads = self.downloads / on_for.num_days() as usize;
        downloads
    }
}

#[derive(Debug, Serialize)]
struct Input {
    inputs: String,
}

async fn create_profile(name: &str, cache: &Cache) -> Result<String, Error> {
    let token = cache.token().ok_or(Error::MissingToken)?;
    let client = Client::new();
    let url = "https://api-inference.huggingface.co/models/black-forest-labs/FLUX.1-schnell";
    let inputs = format!("A cute avatar picture for {name}");
    let json = Input { inputs };
    let response = client
        .post(url)
        .json(&json)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await?;
    let data = response.bytes().await?;
    let mut path = cache.path().clone();
    path.push("profiles");
    if !path.exists() {
        debug!("Attempting to create dir {}", path.display());
        std::fs::create_dir_all(path.clone()).expect("Could not create dir");
    };
    path.push(format!("{}.png", name.replace(' ', "-")));
    info!("Writing avatar into {path:?}");
    let mut file = std::fs::File::create(path.clone())?;
    file.write_all(&data)?;
    Ok(path
        .into_os_string()
        .into_string()
        .expect("Path conversion"))
}

#[tauri::command]
pub async fn get_models(state: tauri::State<'_, State>) -> Result<Vec<model::Model>, Error> {
    debug!("Fetching models");
    let models = model::Entity::find().all(&state.db).await?;
    if !models.is_empty() {
        debug!("Got {} cached models", models.len());
        return Ok(models);
    }
    suggest_models(&state.cache, &state.db).await?;
    let models = model::Entity::find().all(&state.db).await?;
    if models.is_empty() {
        return Err(Error::NoModels);
    }

    return Ok(models);
}

pub async fn suggest_models(cache: &Cache, db: &DatabaseConnection) -> Result<(), Error> {
    let models = model::Entity::find().all(db).await?;
    if !models.is_empty() {
        return Ok(());
    }
    if !cache.token_path().exists() {
        return Err(Error::MissingToken);
    }
    let api = ApiBuilder::new()
        .with_cache_dir(cache.path().clone())
        .build()?;
    // let url = "https://huggingface.co/api/models?inference=warm&pipeline_tag=text-generation&sort=downloads&expand=safetensors";
    let url = "https://huggingface.co/api/models?inference=warm&pipeline_tag=text-generation&sort=downloads";
    let response = api.client().get(url).send().await?.json().await?;
    let mut data: Vec<Model> = serde_json::from_value(response)?;
    info!("Got {} models from API", data.len());
    let now = Utc::now();
    data.sort_by(|a, b| {
        let a = a.score(&now);
        let b = b.score(&now);
        b.cmp(&a)
    });
    let models: Vec<_> = data
        .into_iter()
        .filter_map(|model| {
            let delta = TimeDelta::days(365);
            if now.signed_duration_since(model.created_at) < delta {
                let model_id = model.model_id;
                debug!("Evaluation model {}", model_id);
                let mut split = model_id.splitn(2, '/');
                split.next().expect("Expect a root");
                let name = split.next().expect("Expect a name").replace('-', " ");

                Some(ModelSuggestion {
                    name,
                    full_name: model_id,
                    profile: "".to_string(),
                    // parameters: safetensors.total,
                })
            } else {
                None
            }
        })
        .collect();

    let mut set = JoinSet::new();
    for (i, model) in models.iter().enumerate() {
        let name = model.name.clone();
        let cache = cache.clone();
        set.spawn(async move { (i, create_profile(&name, &cache).await) });
    }
    while let Some(res) = set.join_next().await {
        if let Ok((i, profile)) = res {
            match profile {
                Ok(profile) => {
                    let model = model::ActiveModel {
                        name: Set(models[i].name.clone()),
                        profile: Set(profile),
                        endpoint: Set(format!(
                            "https://api-inference.huggingface.co/models/{}/v1/chat/completions",
                            models[i].full_name
                        )),
                        parameters: Set(model::Parameters::default()),
                        ..Default::default()
                    };
                    model.insert(db).await.unwrap();
                }
                Err(err) => error!("Failed to fetch profile {err:?}"),
            }
        }
    }
    Ok(())
}
