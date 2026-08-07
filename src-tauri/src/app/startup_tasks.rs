//! Usage: Async startup task pipeline extracted from bootstrap setup.

use super::app_state::{ensure_db_ready_for_recovery, DbInitState};
use super::plugins::extension_host_registry::ExtensionHostRuntimeState;
use super::startup_state::{
    fail_startup_run, finish_startup_run, set_startup_stage, try_begin_startup_run, AppStartupStage,
};
#[cfg(test)]
use std::future::Future;
use tauri::Manager;

pub(crate) fn spawn(app_handle: tauri::AppHandle) -> bool {
    if crate::app::maintenance::ensure_normal_operation(&app_handle).is_err() {
        return false;
    }
    if !try_begin_startup_run(&app_handle) {
        return false;
    }
    crate::app::maintenance::begin_recovery_replay(&app_handle);

    tauri::async_runtime::spawn(async move {
        run(app_handle).await;
    });
    true
}

async fn run(app_handle: tauri::AppHandle) {
    let db_state = app_handle.state::<DbInitState>();
    let init_app = app_handle.clone();
    let db = match ensure_db_ready_for_recovery(init_app, db_state.inner()).await {
        Ok(db) => db,
        Err(error) => {
            tracing::error!(code = error.code(), "database initialization for recovery failed");
            crate::app::maintenance::fail_recovery_replay(&app_handle, error);
            return;
        }
    };

    let recovery_app = app_handle.clone();
    let recovery_db = db.clone();
    match crate::blocking::run("startup_recovery_journal", move || {
        crate::infra::recovery_journal::replay_pending(&recovery_app, &recovery_db)
    })
    .await
    {
        Ok(count) => {
            crate::app::maintenance::finish_recovery_replay_for_startup(&app_handle);
            if count > 0 {
                tracing::info!(replayed_count = count, "startup replayed external projections");
            }
        }
        Err(error) => {
            crate::app::maintenance::fail_recovery_replay(&app_handle, error);
            return;
        }
    }

    match crate::request_logs::reconcile_unresolved_pending(
        &db,
        crate::request_logs::RequestLogReconcileReason::StartupRecovery,
        crate::shared::time::now_unix_millis(),
    ) {
        Ok(count) => {
            if count > 0 {
                tracing::info!(
                    reconciled_count = count,
                    "startup reconciled previous-process pending request logs"
                );
            }
        }
        Err(err) => {
            tracing::error!("startup request-log reconciliation failed: {}", err);
            fail_startup_run(
                &app_handle,
                AppStartupStage::InitializingDb,
                format!("请求日志恢复失败：{err}"),
            );
            return;
        }
    }

    let extension_host_state = app_handle.state::<ExtensionHostRuntimeState>();
    match extension_host_state
        .registry(app_handle.clone(), db_state.inner())
        .await
    {
        Ok(registry) => match crate::app::plugin_service::activate_startup_plugins(&db, &registry).await {
            Ok(true) => crate::app::gateway_control::app_refresh_gateway_plugins(&app_handle, &db),
            Ok(false) => {}
            Err(error) => tracing::warn!(
                error = %error,
                "failed to enumerate plugins for startup activation"
            ),
        },
        Err(error) => tracing::warn!(
            error = %error,
            "extension host registry was unavailable for startup activation"
        ),
    }

    crate::request_logs::spawn_retention_task(app_handle.clone(), db.clone());
    crate::domain::provider_availability::spawn_retention_task(db.clone());
    tauri::async_runtime::spawn(crate::app::observer::start_best_effort(app_handle.clone()));

    set_startup_stage(&app_handle, AppStartupStage::ReadingSettings);
    let settings = match crate::app::startup_settings::read(&app_handle).await {
        Ok(settings) => settings,
        Err(err) => {
            fail_startup_run(&app_handle, AppStartupStage::ReadingSettings, err);
            return;
        }
    };

    crate::app::startup_settings::apply_window_state(&app_handle, &settings);

    set_startup_stage(&app_handle, AppStartupStage::StartingGateway);
    let status = match crate::app::startup_gateway::start(&app_handle, db.clone(), &settings).await
    {
        Ok(status) => status,
        Err(err) => {
            fail_startup_run(&app_handle, AppStartupStage::StartingGateway, err);
            return;
        }
    };

    crate::usage_ledger::spawn_backfill(app_handle.clone(), db.clone());

    set_startup_stage(&app_handle, AppStartupStage::SyncingCliProxy);
    crate::app::startup_gateway::sync_cli_proxy_after_autostart(&app_handle, &status).await;

    set_startup_stage(&app_handle, AppStartupStage::FinalizingWsl);
    crate::app::startup_wsl::finalize(&app_handle, db, status.port, settings).await;
    finish_startup_run(&app_handle);
}

#[cfg(test)]
async fn initialize_db_stage<R, F, Fut>(
    app_handle: &tauri::AppHandle<R>,
    initialize: F,
) -> Option<crate::db::Db>
where
    R: tauri::Runtime,
    F: FnOnce() -> Fut,
    Fut: Future<Output = crate::shared::error::AppResult<crate::db::Db>>,
{
    let db = match initialize().await {
        Ok(db) => db,
        Err(err) => {
            tracing::error!("database initialization failed: {}", err);
            fail_startup_run(
                app_handle,
                AppStartupStage::InitializingDb,
                format!("数据库初始化失败：{err}"),
            );
            return None;
        }
    };

    Some(db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::app_state::ensure_db_ready_with;
    use crate::app::startup_state::{startup_status_snapshot, StartupState};
    use crate::shared::error::AppError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn retry_reenters_db_initialization_and_reaches_next_stage() {
        let temp = tempfile::tempdir().expect("tempdir");
        let initialized_db = crate::db::init_for_tests(&temp.path().join("startup-retry.db"))
            .expect("initialize test db");
        let db_state = DbInitState::default();
        let attempts = AtomicUsize::new(0);
        let app = tauri::test::mock_app();
        assert!(app.manage(StartupState::default()));
        let app_handle = app.handle().clone();

        assert!(try_begin_startup_run(&app_handle));
        let first = initialize_db_stage(&app_handle, || {
            ensure_db_ready_with(&db_state, || async {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(AppError::new(
                    "DB_ERROR",
                    "transient initialization failure",
                ))
            })
        })
        .await;
        assert!(first.is_none());
        let failed = startup_status_snapshot(&app_handle);
        assert_eq!(failed.current_stage, AppStartupStage::Failed);
        assert_eq!(failed.failed_stage, Some(AppStartupStage::InitializingDb));
        assert!(failed.can_retry);

        assert!(try_begin_startup_run(&app_handle));
        let second = initialize_db_stage(&app_handle, || {
            ensure_db_ready_with(&db_state, || async {
                attempts.fetch_add(1, Ordering::SeqCst);
                Ok(initialized_db.clone())
            })
        })
        .await;
        assert!(second.is_some());
        assert_eq!(attempts.load(Ordering::SeqCst), 2);

        set_startup_stage(&app_handle, AppStartupStage::ReadingSettings);
        let resumed = startup_status_snapshot(&app_handle);
        assert_eq!(resumed.current_stage, AppStartupStage::ReadingSettings);
        assert_eq!(resumed.failed_stage, None);
        assert!(!resumed.can_retry);
    }
}
