use crate::app_state::{ensure_db_ready, DbInitState};
use crate::shared::ipc_confirm::RiskyIpcConfirm;
use crate::{base_url_probe, blocking, providers};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_copy_api_key_to_clipboard(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    confirm: Option<RiskyIpcConfirm>,
) -> Result<bool, String> {
    RiskyIpcConfirm::require(
        confirm,
        "provider_copy_api_key_to_clipboard",
        format!("provider:{provider_id}:api_key"),
    )?;
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let api_key = blocking::run(
        "provider_copy_api_key_to_clipboard",
        move || -> crate::shared::error::AppResult<String> {
            let conn = db.open_connection()?;
            let provider = providers::get_by_id(&conn, provider_id)?;
            if provider.auth_mode != "api_key" || provider.source_provider_id.is_some() {
                return Err("SEC_INVALID_INPUT: provider does not own a direct api_key"
                    .to_string()
                    .into());
            }

            let api_key = providers::get_api_key_plaintext(&db, provider_id)?;
            if api_key.trim().is_empty() {
                return Err("SEC_INVALID_INPUT: provider api_key is not configured"
                    .to_string()
                    .into());
            }

            Ok(api_key)
        },
    )
    .await?;

    app.clipboard().write_text(api_key).map_err(|err| {
        format!("SYSTEM_ERROR: failed to write provider api_key to clipboard: {err}")
    })?;
    tracing::info!(provider_id, "provider api_key copied to clipboard");
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn base_url_ping_ms(base_url: String) -> Result<u64, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("aio-coding-hub-ping/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("PING_HTTP_CLIENT_INIT: {error}"))?;
    base_url_probe::probe_base_url_ms(&client, &base_url, std::time::Duration::from_secs(3)).await
}
