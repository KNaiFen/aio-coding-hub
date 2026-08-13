//! Usage: Provider sort modes related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::gateway_control::{
    app_gateway_clear_cli_route_runtime_state, app_gateway_clear_cli_session_bindings,
};
use crate::{blocking, sort_modes};

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_modes_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<Vec<sort_modes::SortModeSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("sort_modes_list", move || sort_modes::list_modes(&db))
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_mode_create(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    name: String,
) -> Result<sort_modes::SortModeSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("sort_mode_create", move || {
        sort_modes::create_mode(&db, &name)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_mode_rename(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
    name: String,
) -> Result<sort_modes::SortModeSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("sort_mode_rename", move || {
        sort_modes::rename_mode(&db, mode_id, &name)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_mode_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let affected_cli_keys = blocking::run("sort_mode_delete", move || {
        sort_modes::delete_mode_with_affected_cli_keys(&db, mode_id)
    })
    .await?;

    for cli_key in affected_cli_keys {
        app_gateway_clear_cli_route_runtime_state(&app, &cli_key);
    }

    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_mode_active_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<Vec<sort_modes::SortModeActiveRow>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("sort_mode_active_list", move || {
        sort_modes::list_active(&db)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_mode_active_set(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    mode_id: Option<i64>,
) -> Result<sort_modes::SortModeActiveRow, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let cli_key_for_db = cli_key.clone();
    let row = blocking::run("sort_mode_active_set", move || {
        sort_modes::set_active(&db, &cli_key_for_db, mode_id)
    })
    .await?;

    app_gateway_clear_cli_route_runtime_state(&app, &cli_key);

    Ok(row)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_mode_providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
    cli_key: String,
) -> Result<Vec<sort_modes::SortModeProviderRow>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("sort_mode_providers_list", move || {
        sort_modes::list_mode_providers(&db, mode_id, &cli_key)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_mode_providers_set_order(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<sort_modes::SortModeProviderRow>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let cli_key_for_db = cli_key.clone();
    let result = blocking::run("sort_mode_providers_set_order", move || {
        sort_modes::set_mode_providers_order(&db, mode_id, &cli_key_for_db, ordered_provider_ids)
    })
    .await
    .map_err(Into::into);

    if result.is_ok() {
        app_gateway_clear_cli_route_runtime_state(&app, &cli_key);
    }

    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_mode_provider_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
    cli_key: String,
    provider_id: i64,
    enabled: bool,
) -> Result<sort_modes::SortModeProviderRow, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let cli_key_for_db = cli_key.clone();
    let result = blocking::run("sort_mode_provider_set_enabled", move || {
        sort_modes::set_mode_provider_enabled(&db, mode_id, &cli_key_for_db, provider_id, enabled)
    })
    .await
    .map_err(Into::into);

    if result.is_ok() {
        app_gateway_clear_cli_route_runtime_state(&app, &cli_key);
    }

    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn sort_mode_provider_set_session_reuse_priority(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
    cli_key: String,
    provider_id: i64,
    session_reuse_priority: i64,
) -> Result<sort_modes::SortModeProviderRow, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let cli_key_for_db = cli_key.clone();
    let result = blocking::run("sort_mode_provider_set_session_reuse_priority", move || {
        sort_modes::set_mode_provider_session_reuse_priority(
            &db,
            mode_id,
            &cli_key_for_db,
            provider_id,
            session_reuse_priority,
        )
    })
    .await
    .map_err(Into::into);

    if result.is_ok() {
        let cleared = app_gateway_clear_cli_session_bindings(&app, &cli_key);
        tracing::info!(
            cli_key = %cli_key,
            mode_id,
            provider_id,
            session_reuse_priority,
            cleared_session_bindings = cleared,
            "sort mode session reuse priority updated"
        );
    }

    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_model_routing_policy_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    provider_uuid: String,
    mode_id: Option<i64>,
    mode_uuid: Option<String>,
) -> Result<sort_modes::ProviderModelRoutingPolicyView, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("provider_model_routing_policy_get", move || {
        sort_modes::provider_model_routing_policy_get(
            &db,
            provider_id,
            &provider_uuid,
            mode_id,
            mode_uuid.as_deref(),
        )
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_model_routing_policy_save(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: sort_modes::ProviderModelRoutingPolicySaveInput,
) -> Result<sort_modes::ProviderModelRoutingPolicyView, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let cli_key = blocking::run("provider_model_routing_policy_save_cli", {
        let db = db.clone();
        let provider_id = input.provider_id;
        move || crate::providers::cli_key_by_id(&db, provider_id)
    })
    .await?;
    let result = blocking::run("provider_model_routing_policy_save", move || {
        sort_modes::provider_model_routing_policy_save(&db, input)
    })
    .await?;
    app_gateway_clear_cli_route_runtime_state(&app, &cli_key);
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn routing_provider_candidates_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
    mode_uuid: String,
    cli_key: String,
) -> Result<Vec<sort_modes::RoutingProviderCandidate>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("routing_provider_candidates_list", move || {
        sort_modes::routing_provider_candidates_list(&db, mode_id, &mode_uuid, &cli_key)
    })
    .await
    .map_err(Into::into)
}
