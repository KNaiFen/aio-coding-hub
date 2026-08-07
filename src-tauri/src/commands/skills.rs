//! Usage: Skills management related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::infra::recovery_journal::{self, JournalContext};
use crate::shared::cli_key::CliKey;
use crate::shared::ipc_confirm::RiskyIpcConfirm;
use crate::{blocking, skills};

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_repos_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<Vec<skills::SkillRepoSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("skill_repos_list", move || skills::repos_list(&db))
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_repo_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    repo_id: Option<i64>,
    git_url: String,
    branch: String,
    enabled: bool,
) -> Result<skills::SkillRepoSummary, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("skill_repo_upsert", move || {
        skills::repo_upsert(&db, repo_id, &git_url, &branch, enabled)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_repo_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    repo_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run(
        "skill_repo_delete",
        move || -> crate::shared::error::AppResult<bool> {
            skills::repo_delete(&db, repo_id)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_installed_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<skills::InstalledSkillSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("skills_installed_list", move || {
        skills::installed_list_for_workspace(&db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_discover_available(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    refresh: bool,
) -> Result<Vec<skills::AvailableSkillSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("skills_discover_available", move || {
        skills::discover_available(&app, &db, refresh)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_repo_discover_available(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    repo_id: i64,
    refresh: bool,
) -> Result<Vec<skills::AvailableSkillSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("skill_repo_discover_available", move || {
        skills::discover_repo_available(&app, &db, repo_id, refresh)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn skill_install(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    git_url: String,
    branch: String,
    source_subdir: String,
    enabled: bool,
) -> Result<skills::InstalledSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    recovery_journal::run_blocking_operation(
        "skill_install",
        app.clone(),
        db.clone(),
        "skill.install",
        JournalContext::for_workspace(workspace_id),
        move |operation| {
            skills::install(
                &app,
                &db,
                workspace_id,
                &git_url,
                &branch,
                &source_subdir,
                enabled,
                operation,
            )
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_install_to_local(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    git_url: String,
    branch: String,
    source_subdir: String,
) -> Result<skills::LocalSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    recovery_journal::run_blocking_operation(
        "skill_install_to_local",
        app.clone(),
        db.clone(),
        "skill.install_to_local",
        JournalContext::for_workspace(workspace_id),
        move |operation| {
            skills::install_to_local(
                &app,
                &db,
                workspace_id,
                &git_url,
                &branch,
                &source_subdir,
                operation,
            )
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    skill_id: i64,
    enabled: bool,
) -> Result<skills::InstalledSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    recovery_journal::run_blocking_operation(
        "skill_set_enabled",
        app.clone(),
        db.clone(),
        "skill.set_enabled",
        JournalContext {
            workspace_id: Some(workspace_id),
            entity_id: Some(skill_id),
            ..JournalContext::default()
        },
        move |operation| {
            skills::set_enabled(&app, &db, workspace_id, skill_id, enabled, operation)
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_uninstall(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    skill_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    recovery_journal::run_blocking_operation(
        "skill_uninstall",
        app.clone(),
        db.clone(),
        "skill.uninstall",
        JournalContext::for_entity(skill_id),
        move |operation| -> crate::shared::error::AppResult<bool> {
            skills::uninstall(&app, &db, skill_id, operation)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_return_to_local(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    skill_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    recovery_journal::run_blocking_operation(
        "skill_return_to_local",
        app.clone(),
        db.clone(),
        "skill.return_to_local",
        JournalContext {
            workspace_id: Some(workspace_id),
            entity_id: Some(skill_id),
            ..JournalContext::default()
        },
        move |operation| -> crate::shared::error::AppResult<bool> {
            skills::return_to_local(&app, &db, workspace_id, skill_id, operation)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_local_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<skills::LocalSkillSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("skills_local_list", move || {
        skills::local_list(&app, &db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_local_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    dir_name: String,
    confirm: Option<RiskyIpcConfirm>,
) -> Result<bool, String> {
    RiskyIpcConfirm::require(
        confirm,
        "skill_local_delete",
        format!("workspace:{workspace_id}:skill-local:{dir_name}"),
    )?;
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    recovery_journal::run_blocking_operation(
        "skill_local_delete",
        app.clone(),
        db.clone(),
        "skill.local_delete",
        JournalContext::for_workspace(workspace_id),
        move |operation| -> crate::shared::error::AppResult<bool> {
            skills::delete_local(&app, &db, workspace_id, &dir_name, operation)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_import_local(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    dir_name: String,
) -> Result<skills::InstalledSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    recovery_journal::run_blocking_operation(
        "skill_import_local",
        app.clone(),
        db.clone(),
        "skill.import_local",
        JournalContext::for_workspace(workspace_id),
        move |operation| skills::import_local(&app, &db, workspace_id, &dir_name, operation),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_import_local_batch(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    dir_names: Vec<String>,
) -> Result<skills::SkillImportLocalBatchReport, String> {
    if dir_names.is_empty() {
        return Err("SEC_INVALID_INPUT: dir_names is required".to_string());
    }
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for dir_name in dir_names {
        let trimmed = dir_name.trim().to_string();
        if trimmed.is_empty() {
            skipped.push(skills::SkillImportIssue {
                dir_name,
                error_code: Some("SEC_INVALID_INPUT".to_string()),
                message: "SEC_INVALID_INPUT: dir_name is required".to_string(),
            });
            continue;
        }

        let operation_app = app.clone();
        let operation_db = db.clone();
        let operation_dir_name = trimmed.clone();
        match recovery_journal::run_blocking_operation(
            "skills_import_local_batch_item",
            app.clone(),
            db.clone(),
            "skill.import_local",
            JournalContext::for_workspace(workspace_id),
            move |operation| {
                skills::import_local(
                    &operation_app,
                    &operation_db,
                    workspace_id,
                    &operation_dir_name,
                    operation,
                )
            },
        )
        .await
        {
            Ok(row) => imported.push(row),
            Err(error) => {
                let message = error.to_string();
                let error_code = message
                    .split(':')
                    .next()
                    .map(str::trim)
                    .filter(|code| !code.is_empty())
                    .map(ToString::to_string);
                let issue = skills::SkillImportIssue {
                    dir_name: trimmed,
                    error_code,
                    message: message.clone(),
                };
                if message.starts_with("SKILL_IMPORT_CONFLICT")
                    || message.starts_with("SKILL_ALREADY_MANAGED")
                    || message.starts_with("SKILL_LOCAL_NOT_FOUND")
                    || message.starts_with("SEC_INVALID_INPUT")
                {
                    skipped.push(issue);
                } else {
                    failed.push(issue);
                }
            }
        }
    }

    Ok(skills::SkillImportLocalBatchReport {
        imported,
        skipped,
        failed,
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_paths_get(
    app: tauri::AppHandle,
    cli_key: String,
) -> Result<skills::SkillsPaths, String> {
    let cli_key = normalize_skills_cli_key(&cli_key)?;

    blocking::run("skills_paths_get", move || {
        skills::paths_get(&app, &cli_key)
    })
    .await
    .map_err(Into::into)
}

fn normalize_skills_cli_key(cli_key: &str) -> Result<String, String> {
    Ok(CliKey::parse(cli_key.trim())
        .map_err(String::from)?
        .to_string())
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_check_updates(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<skills::SkillUpdateInfo>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("skill_check_updates", move || {
        skills::check_updates_for_workspace(&app, &db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_skills_cli_key_trims_supported_keys() {
        assert_eq!(
            normalize_skills_cli_key(" claude ").expect("valid cli key"),
            "claude"
        );
    }

    #[test]
    fn normalize_skills_cli_key_rejects_invalid_keys() {
        let err = normalize_skills_cli_key(" opencode ").expect_err("invalid cli key");
        assert_eq!(err, "SEC_INVALID_INPUT: unknown cli_key=opencode");
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_update(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    skill_id: i64,
) -> Result<skills::InstalledSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    recovery_journal::run_blocking_operation(
        "skill_update",
        app.clone(),
        db.clone(),
        "skill.update",
        JournalContext {
            workspace_id: Some(workspace_id),
            entity_id: Some(skill_id),
            ..JournalContext::default()
        },
        move |operation| skills::update_skill(&app, &db, workspace_id, skill_id, operation),
    )
    .await
    .map_err(Into::into)
}
