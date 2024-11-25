use crate::{
    entities::{model, user},
    State,
};
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

#[derive(Serialize)]
pub struct ModelItem {
    id: u32,
    name: String,
    profile: String,
}

#[tauri::command]
pub async fn get_models(state: tauri::State<'_, State>) -> Result<Vec<ModelItem>, Error> {
    debug!("Fetching models");
    let models = model::Entity::find()
        .find_also_related(user::Entity)
        .all(&state.db)
        .await?;
    let models = if !models.is_empty() {
        debug!("Got {} cached models", models.len());
        models
    } else {
        suggest_models(&state.cache, &state.db).await?;
        let models = model::Entity::find()
            .find_also_related(user::Entity)
            .all(&state.db)
            .await?;
        models
    };

    if models.is_empty() {
        return Err(Error::NoModels);
    }

    let models = models
        .into_iter()
        .map(|(m, ou)| {
            let u = ou.expect("User for model");
            ModelItem {
                id: m.id,
                name: u.name,
                profile: u.profile,
            }
        })
        .collect();
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
                    profile: "public/default_profile.png".to_string(),
                    // parameters: safetensors.total,
                })
            } else {
                None
            }
        })
        .take(10)
        .collect();

    for sugg in models {
        let user = user::ActiveModel {
            name: Set(sugg.name.clone()),
            profile: Set(sugg.profile.clone()),
            ..Default::default()
        };
        let user: user::Model = user.insert(db).await?;
        let model = model::ActiveModel {
            user_id: Set(user.id),
            endpoint: Set(format!(
                "https://api-inference.huggingface.co/models/{}/v1/chat/completions",
                sugg.full_name
            )),
            parameters: Set(model::Parameters::default()),
            ..Default::default()
        };
        model.insert(db).await.unwrap();
        let name = sugg.name.clone();
        let cache = cache.clone();

        let db_clone = db.clone();
        tokio::spawn(async move {
            let profile = create_profile(&name, &cache)
                .await
                .expect("Creating profile");
            let user: user::Model = user::Entity::find_by_id(user.id)
                .one(&db_clone)
                .await
                .expect("Valid query")
                .expect("Model user for profile");
            // Into ActiveModel
            let mut user: user::ActiveModel = user.into();
            // Update name attribute
            user.profile = Set(profile);
            user.update(&db_clone).await.expect("Profile update");
        });
    }

    Ok(())
}
