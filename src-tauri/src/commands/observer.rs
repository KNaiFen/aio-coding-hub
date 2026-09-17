use crate::app::observer::lan::{self, ObserverLanStatus};

#[tauri::command]
#[specta::specta]
pub(crate) async fn observer_lan_status(app: tauri::AppHandle) -> Result<ObserverLanStatus, String> {
    lan::status(app).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn observer_lan_configure(app: tauri::AppHandle, enabled: bool, port: u16) -> Result<ObserverLanStatus, String> {
    lan::configure(app, enabled, port).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn observer_lan_token_reveal(app: tauri::AppHandle) -> Result<String, String> {
    lan::token(app, false).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn observer_lan_token_rotate(app: tauri::AppHandle) -> Result<String, String> {
    lan::token(app, true).await
}
