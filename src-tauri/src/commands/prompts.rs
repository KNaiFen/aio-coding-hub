//! Usage: Prompt templates related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::infra::recovery_journal::{self, JournalContext};
use crate::{blocking, prompts};

#[tauri::command]
#[specta::specta]
pub(crate) async fn prompts_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<prompts::PromptSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("prompts_list", move || {
        prompts::list_by_workspace(&db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn prompts_list_summary(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<prompts::PromptListSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("prompts_list_summary", move || {
        prompts::list_summaries_by_workspace(&db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn prompts_default_sync_from_files(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<prompts::DefaultPromptSyncReport, String> {
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = recovery_journal::run_blocking_operation(
        "prompts_default_sync_from_files",
        app.clone(),
        db.clone(),
        "prompt.default_sync",
        JournalContext::default(),
        move |_operation| prompts::default_sync_from_files(&app, &db),
    )
    .await
    .map_err(Into::into);
    #[cfg(windows)]
    if result.is_ok() {
        super::wsl::wsl_sync_trigger::trigger(app_for_wsl);
    }
    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn prompt_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    prompt_id: Option<i64>,
    workspace_id: i64,
    name: String,
    content: String,
    enabled: bool,
) -> Result<prompts::PromptSummary, String> {
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let context = JournalContext {
        workspace_id: Some(workspace_id),
        entity_id: prompt_id,
        ..JournalContext::default()
    };
    let result = recovery_journal::run_blocking_operation(
        "prompt_upsert",
        app.clone(),
        db.clone(),
        "prompt.upsert",
        context,
        move |operation| {
            prompts::upsert(
                &db,
                prompt_id,
                workspace_id,
                &name,
                &content,
                enabled,
                operation,
            )
        },
    )
    .await
    .map_err(Into::into);
    #[cfg(windows)]
    if result.is_ok() {
        super::wsl::wsl_sync_trigger::trigger(app_for_wsl);
    }
    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn prompt_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    prompt_id: i64,
    enabled: bool,
) -> Result<prompts::PromptSummary, String> {
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = recovery_journal::run_blocking_operation(
        "prompt_set_enabled",
        app.clone(),
        db.clone(),
        "prompt.set_enabled",
        JournalContext::for_entity(prompt_id),
        move |operation| prompts::set_enabled(&app, &db, prompt_id, enabled, operation),
    )
    .await
    .map_err(Into::into);
    #[cfg(windows)]
    if result.is_ok() {
        super::wsl::wsl_sync_trigger::trigger(app_for_wsl);
    }
    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn prompt_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    prompt_id: i64,
) -> Result<bool, String> {
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = recovery_journal::run_blocking_operation(
        "prompt_delete",
        app.clone(),
        db.clone(),
        "prompt.delete",
        JournalContext::for_entity(prompt_id),
        move |operation| -> crate::shared::error::AppResult<bool> {
            prompts::delete(&app, &db, prompt_id, operation)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into);
    #[cfg(windows)]
    if result.is_ok() {
        super::wsl::wsl_sync_trigger::trigger(app_for_wsl);
    }
    result
}
