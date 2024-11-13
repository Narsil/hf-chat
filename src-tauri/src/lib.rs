mod commands;
mod entities;
pub mod migrations;

use crate::commands::api::Stream;
use crate::commands::login::Openid;
use hf_hub::Cache;
use log::{debug, info, warn};
use sea_orm::{Database, DatabaseConnection};
use std::path::Path;
use tauri::Manager;
use tokio::sync::Mutex;

struct State {
    db: DatabaseConnection,
    cache: Cache,
    // device: Device,
    openid: Mutex<Option<Openid>>,
    // tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    stream: Mutex<Option<Stream>>,
}

fn cache(path: &Path) -> Cache {
    let cache = {
        std::fs::create_dir_all(path).expect("Could not create dir");
        let cache = Cache::new(path.to_path_buf());
        cache
    };
    cache
}

async fn init_db(cache: &Cache) -> Result<DatabaseConnection, sea_orm::DbErr> {
    let mut path = cache.path().clone();
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .filter(|metadata| metadata.target().starts_with("hf_chat_lib"))
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::load::load,
            commands::login::login,
            commands::login::login_callback,
            commands::models::get_models,
            commands::conversation::create_conversation,
            commands::conversation::new_message,
            commands::conversation::get_messages,
            commands::api::get_chunk,
        ])
        .setup(move |app| {
            info!("Start the run");
            // info!(
            //     "avx: {}, neon: {}, simd128: {}, f16c: {}",
            //     candle::utils::with_avx(),
            //     candle::utils::with_neon(),
            //     candle::utils::with_simd128(),
            //     candle::utils::with_f16c()
            // );
            let mut path = app.path().app_data_dir().expect("Have a local data dir");
            path.push("chat");
            let cache = cache(&path);
            log::info!("Start the db");
            let db = tauri::async_runtime::block_on(async {
                init_db(&cache).await.expect("Failed to create db")
            });

            let db2 = db.clone();
            let cache2 = cache.clone();
            tauri::async_runtime::spawn(async move {
                match commands::models::suggest_models(&cache2, &db2).await {
                    Ok(_) => info!("Loaded model suggestions"),
                    Err(err) => warn!("Ignored model suggestions {err:?}"),
                }
            });
            info!("get the device");
            // let device = if candle::utils::cuda_is_available() {
            //     Device::new_cuda(0)?
            // // Simulator doesn't support MPS (Metal Performance Shader).
            // } else if candle::utils::metal_is_available() && TARGET != "aarch64-apple-ios-sim" {
            //     Device::new_metal(0)?
            // } else {
            //     Device::Cpu
            // };
            app.manage(State {
                db,
                cache,
                // device,
                openid: Mutex::new(None),
                stream: Mutex::new(None),
                // tx: Mutex::new(None),
            });
            // if let Some(setup) = setup {
            //     (setup)(app)?;
            // }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
