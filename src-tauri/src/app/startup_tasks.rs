//! Usage: Async startup task pipeline extracted from bootstrap setup.

use super::app_state::{ensure_db_ready, DbInitState};
use super::plugins::extension_host_registry::ExtensionHostRuntimeState;
use super::startup_state::{
    fail_startup_run, finish_startup_run, set_startup_stage, try_begin_startup_run, AppStartupStage,
};
use std::future::Future;
use tauri::Manager;

pub(crate) fn spawn(app_handle: tauri::AppHandle) -> bool {
    if crate::app::maintenance::ensure_normal_operation(&app_handle).is_err() {
        return false;
    }
    if !try_begin_startup_run(&app_handle) {
        return false;
    }
    tauri::async_runtime::spawn(async move {
        run(app_handle).await;
    });
    true
}

async fn run(app_handle: tauri::AppHandle) {
    let db_state = app_handle.state::<DbInitState>();
    let init_app = app_handle.clone();
    let db = match initialize_db_stage(&app_handle, || ensure_db_ready(init_app, db_state.inner()))
        .await
    {
        Some(db) => db,
        None => return,
    };

    let app_for_replay = app_handle.clone();
    let db_for_replay = db.clone();
    if !replay_recovery_journal_stage(&app_handle, move || {
        crate::infra::recovery_journal::replay_pending(&app_for_replay, &db_for_replay)
    })
    .await
    {
        return;
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
        Ok(registry) => match crate::app::plugin_service::activate_startup_plugins(&db, &registry)
            .await
        {
            Ok(quarantined_plugin_ids) => {
                for plugin_id in &quarantined_plugin_ids {
                    crate::app::gateway_control::app_remove_gateway_plugin(&app_handle, plugin_id);
                }
                if !quarantined_plugin_ids.is_empty() {
                    crate::app::gateway_control::app_refresh_gateway_plugins(&app_handle, &db);
                }
            }
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
    if let Some(runtime) = app_handle.try_state::<
        crate::app::provider_availability_probe_runtime::ProviderAvailabilityProbeRuntimeState,
    >() {
        runtime.start_scheduler(app_handle.clone(), db.clone());
    }
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

async fn replay_recovery_journal_stage<R, F>(app_handle: &tauri::AppHandle<R>, replay: F) -> bool
where
    R: tauri::Runtime,
    F: FnOnce() -> crate::shared::error::AppResult<usize> + Send + 'static,
{
    crate::app::maintenance::begin_recovery_replay(app_handle);
    match crate::blocking::run("recovery_journal_startup", replay).await {
        Ok(count) => {
            if count > 0 {
                tracing::info!(
                    reconciled_count = count,
                    "startup reconciled pending external-effect recovery journals"
                );
            }
            crate::app::maintenance::finish_recovery_replay_for_startup(app_handle);
            true
        }
        Err(error) => {
            crate::app::maintenance::fail_recovery_replay(app_handle, error);
            false
        }
    }
}

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
    use crate::app::maintenance::MaintenanceState;
    use crate::app::startup_state::{startup_status_snapshot, AppStartupStage, StartupState};
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

    #[tokio::test]
    async fn recovery_replay_resumes_startup_only_after_success() {
        let app = tauri::test::mock_app();
        assert!(app.manage(MaintenanceState::default()));
        assert!(app.manage(StartupState::default()));
        let app_handle = app.handle().clone();

        assert!(try_begin_startup_run(&app_handle));
        assert!(replay_recovery_journal_stage(&app_handle, || Ok::<_, AppError>(1)).await);

        assert!(!app_handle
            .state::<MaintenanceState>()
            .blocks_normal_operation());
        let status = startup_status_snapshot(&app_handle);
        assert!(status.running);
        assert!(!status.maintenance_mode);
        assert_eq!(status.current_stage, AppStartupStage::InitializingDb);
    }

    #[tokio::test]
    async fn recovery_replay_failure_keeps_maintenance_gate_closed() {
        let app = tauri::test::mock_app();
        assert!(app.manage(MaintenanceState::default()));
        assert!(app.manage(StartupState::default()));
        let app_handle = app.handle().clone();

        assert!(try_begin_startup_run(&app_handle));
        assert!(
            !replay_recovery_journal_stage(&app_handle, || {
                Err::<usize, _>(AppError::new("RECOVERY_REPLAY_FAILED", "test failure"))
            })
            .await
        );

        assert!(app_handle
            .state::<MaintenanceState>()
            .blocks_normal_operation());
        let status = startup_status_snapshot(&app_handle);
        assert!(!status.running);
        assert!(status.maintenance_mode);
        assert_eq!(status.current_stage, AppStartupStage::Failed);
        assert_eq!(status.failed_stage, Some(AppStartupStage::InitializingDb));
        assert!(status.can_retry);
    }
}
