//! Usage: Provider availability test Tauri command.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::domain::provider_availability;
use crate::{blocking, settings};
use tauri::Manager;

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_test_availability(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<provider_availability::ProviderAvailabilityResult, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let Some(runtime) = app.try_state::<
        crate::app::provider_availability_probe_runtime::ProviderAvailabilityProbeRuntimeState,
    >() else {
        return Err("SYSTEM_ERROR: provider availability runtime is unavailable".to_string());
    };
    runtime
        .probe_manual(app.clone(), db, provider_id)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_availability_timelines_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_ids: Vec<i64>,
    bucket_count: u16,
) -> Result<Vec<provider_availability::ProviderAvailabilityTimeline>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let hours = settings::read(&app)
        .map_err(String::from)?
        .provider_availability_hours;
    blocking::run("provider_availability_timelines_get", move || {
        provider_availability::timelines(
            &db,
            &provider_ids,
            hours,
            bucket_count,
            crate::shared::time::now_unix_millis(),
        )
    })
    .await
    .map_err(Into::into)
}
