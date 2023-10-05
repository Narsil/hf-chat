#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[tokio::main]
pub async fn main() {
    chat::AppBuilder::new().run().await;
}
