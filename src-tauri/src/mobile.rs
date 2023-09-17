#[tauri::mobile_entry_point]
#[tokio::main]
async fn main() {
    super::AppBuilder::new().run().await
}
