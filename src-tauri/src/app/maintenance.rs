//! Usage: Application-level maintenance coordinator for durable reset work.
//!
//! Reset is deliberately split across process boundaries.  The request path
//! only records intent and exits; the next process consumes the marker before
//! it creates any normal runtime owner.

use crate::shared::error::{AppError, AppResult};
use crate::shared::fs::{
    read_optional_file_with_max_len, rename_file_no_replace, write_file_atomic_create_new,
};
use crate::shared::mutex_ext::MutexExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Mutex;
use tauri::Manager;

const MAINTENANCE_DIR: &str = ".maintenance";
const RESET_MARKER_FILE: &str = "reset-app-data.pending";
const RESET_COMPLETED_FILE: &str = "reset-app-data.completed";
const RESET_MARKER_CONTENT: &[u8] = b"aio-coding-hub-reset-app-data-v1\n";
const RESET_MARKER_MAX_BYTES: usize = 128;
const MAINTENANCE_CLEAN: u8 = 0;
const MAINTENANCE_RUNNING: u8 = 1;
const MAINTENANCE_FAILED: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResetMarkerState {
    None,
    Pending,
    Completed,
}

#[derive(Default)]
pub(crate) struct MaintenanceState {
    phase: AtomicU8,
    reset_exit_requested: AtomicBool,
    runtime_started: AtomicBool,
    coordinator_lock: Mutex<()>,
}

impl MaintenanceState {
    fn phase(&self) -> u8 {
        self.phase.load(Ordering::Acquire)
    }

    fn set_phase(&self, phase: u8) {
        self.phase.store(phase, Ordering::Release);
    }

    pub(crate) fn blocks_normal_operation(&self) -> bool {
        self.phase() != MAINTENANCE_CLEAN
    }

    fn can_retry(&self) -> bool {
        self.phase() == MAINTENANCE_FAILED && !self.reset_exit_requested.load(Ordering::Acquire)
    }

    fn try_begin_retry(&self) -> bool {
        if self.reset_exit_requested.load(Ordering::Acquire) {
            return false;
        }
        self.phase
            .compare_exchange(
                MAINTENANCE_FAILED,
                MAINTENANCE_RUNNING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn allows_invoke(&self, command: &str) -> bool {
        if !self.blocks_normal_operation() {
            return true;
        }

        match command {
            "app_startup_status_get" | "app_exit" => true,
            "app_startup_retry" => self.can_retry(),
            _ => false,
        }
    }

    pub(crate) fn request_reset_exit(&self) {
        self.set_phase(MAINTENANCE_RUNNING);
        self.reset_exit_requested.store(true, Ordering::Release);
    }

    pub(crate) fn should_skip_exit_cleanup(&self) -> bool {
        self.reset_exit_requested.load(Ordering::Acquire) || self.blocks_normal_operation()
    }

    fn try_mark_runtime_started(&self) -> bool {
        if self.blocks_normal_operation() {
            return false;
        }
        self.runtime_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn lock_coordinator(&self) -> std::sync::MutexGuard<'_, ()> {
        self.coordinator_lock.lock_or_recover()
    }
}

pub(crate) fn ensure_normal_operation<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<()> {
    let Some(state) = app.try_state::<MaintenanceState>() else {
        return Ok(());
    };
    if state.blocks_normal_operation() {
        return Err(AppError::new(
            "APP_MAINTENANCE_REQUIRED",
            "应用正在维护中，请重试数据清理或退出",
        ));
    }
    Ok(())
}

pub(crate) fn invoke_allowed_during_maintenance<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    command: &str,
) -> bool {
    app.try_state::<MaintenanceState>()
        .is_none_or(|state| state.allows_invoke(command))
}

pub(crate) fn marker_path_for_data_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(MAINTENANCE_DIR).join(RESET_MARKER_FILE)
}

fn completed_marker_path_for_data_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(MAINTENANCE_DIR).join(RESET_COMPLETED_FILE)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(path)?.sync_all()
}

#[cfg(windows)]
fn sync_directory(path: &Path) -> std::io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;

    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?
        .sync_all()
}

#[cfg(not(any(unix, windows)))]
fn sync_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn sync_directory_for_reset(path: &Path) -> AppResult<()> {
    sync_directory(path)
        .map_err(|_| AppError::new("APP_MAINTENANCE_MARKER_FAILED", "数据重置持久化未完成"))
}

fn sync_parent_directory(path: &Path) -> AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::new("APP_MAINTENANCE_MARKER_FAILED", "维护 marker 路径无效"))?;
    sync_directory_for_reset(parent)
}

fn sync_reset_target_directories(data_dir: &Path, db_path: &Path) -> AppResult<()> {
    sync_directory_for_reset(data_dir)?;
    if let Some(db_parent) = db_path.parent() {
        if db_parent != data_dir {
            sync_directory_for_reset(db_parent)?;
        }
    }
    Ok(())
}

fn marker_bytes(path: &Path) -> AppResult<Option<Vec<u8>>> {
    read_optional_file_with_max_len(path, RESET_MARKER_MAX_BYTES)
}

fn marker_is_pending(path: &Path) -> AppResult<bool> {
    match marker_bytes(path)? {
        None => Ok(false),
        Some(bytes) if bytes == RESET_MARKER_CONTENT => Ok(true),
        Some(_) => Err(AppError::new(
            "APP_MAINTENANCE_MARKER_INVALID",
            "维护 marker 无法验证",
        )),
    }
}

fn reset_marker_state(data_dir: &Path) -> AppResult<ResetMarkerState> {
    let pending = marker_path_for_data_dir(data_dir);
    if marker_is_pending(&pending)? {
        return Ok(ResetMarkerState::Pending);
    }

    let completed = completed_marker_path_for_data_dir(data_dir);
    if marker_is_pending(&completed)? {
        return Ok(ResetMarkerState::Completed);
    }
    Ok(ResetMarkerState::None)
}

fn remove_completed_marker_if_present(data_dir: &Path) -> AppResult<()> {
    let completed = completed_marker_path_for_data_dir(data_dir);
    if !marker_is_pending(&completed)? {
        return Ok(());
    }
    match std::fs::remove_file(&completed) {
        Ok(()) => sync_parent_directory(&completed),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(AppError::new(
            "APP_MAINTENANCE_MARKER_FAILED",
            "无法清理数据重置完成标记",
        )),
    }
}

/// Persist the reset intent exactly once.  A second request validates the
/// existing marker and is treated as an idempotent success.
pub(crate) fn write_reset_marker_at(data_dir: &Path) -> AppResult<bool> {
    let marker = marker_path_for_data_dir(data_dir);
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|_| AppError::new("APP_MAINTENANCE_MARKER_FAILED", "无法登记数据重置"))?;
        // Persist both the app-data directory entry and the maintenance
        // directory entry before relying on the marker inside it.
        if let Some(data_parent) = data_dir.parent() {
            sync_directory_for_reset(data_parent)?;
        }
        sync_directory_for_reset(data_dir)?;
    }

    remove_completed_marker_if_present(data_dir)?;

    match write_file_atomic_create_new(&marker, RESET_MARKER_CONTENT) {
        Ok(()) => {
            sync_parent_directory(&marker)?;
            Ok(true)
        }
        Err(error) if error.code() == "FS_ALREADY_EXISTS" => {
            if marker_is_pending(&marker)? {
                Ok(false)
            } else {
                Err(AppError::new(
                    "APP_MAINTENANCE_MARKER_INVALID",
                    "维护 marker 无法验证",
                ))
            }
        }
        Err(_) => Err(AppError::new(
            "APP_MAINTENANCE_MARKER_FAILED",
            "无法登记数据重置",
        )),
    }
}

fn remove_reset_marker_at(data_dir: &Path) -> AppResult<()> {
    let marker = marker_path_for_data_dir(data_dir);
    let completed = completed_marker_path_for_data_dir(data_dir);
    remove_completed_marker_if_present(data_dir)?;

    match rename_file_no_replace(&marker, &completed) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => {
            return Err(AppError::new(
                "APP_MAINTENANCE_MARKER_FAILED",
                "数据已清理，但维护 marker 无法完成",
            ));
        }
    }

    // The durable rename guarantees that a crash exposes either Pending or
    // Completed. Removing the tombstone afterwards is only housekeeping.
    sync_parent_directory(&completed)?;
    let _ = remove_completed_marker_if_present(data_dir);
    Ok(())
}

pub(crate) fn consume_reset_marker_at(data_dir: &Path, db_path: &Path) -> AppResult<bool> {
    match reset_marker_state(data_dir)? {
        ResetMarkerState::None => return Ok(false),
        ResetMarkerState::Completed => {
            let _ = remove_completed_marker_if_present(data_dir);
            return Ok(true);
        }
        ResetMarkerState::Pending => {}
    }

    crate::infra::data_management::app_data_reset_at(data_dir, db_path)?;
    // The target unlinks must be durable before the marker unlink becomes
    // durable. Otherwise a power loss could restore old data without a marker.
    sync_reset_target_directories(data_dir, db_path)?;
    remove_reset_marker_at(data_dir)?;
    Ok(true)
}

fn maintenance_failure_message(error: &AppError) -> String {
    // Keep filesystem paths and OS error strings out of the startup IPC/UI.
    format!("数据重置未完成（{}），只能重试或退出", error.code())
}

fn recovery_failure_message(error: &AppError) -> String {
    format!("外部配置对账未完成（{}），只能重试或退出", error.code())
}

fn begin_maintenance<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(state) = app.try_state::<MaintenanceState>() {
        state.set_phase(MAINTENANCE_RUNNING);
    }
    crate::app::startup_state::begin_maintenance_run(app);
}

fn fail_maintenance<R: tauri::Runtime>(app: &tauri::AppHandle<R>, error: AppError) {
    if let Some(state) = app.try_state::<MaintenanceState>() {
        state.set_phase(MAINTENANCE_FAILED);
    }
    crate::app::startup_state::fail_maintenance_run(app, maintenance_failure_message(&error));
}

fn finish_maintenance<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(state) = app.try_state::<MaintenanceState>() {
        state.reset_exit_requested.store(false, Ordering::Release);
        state.set_phase(MAINTENANCE_CLEAN);
    }
    crate::app::startup_state::finish_maintenance_run(app);
}

fn finish_maintenance_for_startup<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(state) = app.try_state::<MaintenanceState>() {
        state.reset_exit_requested.store(false, Ordering::Release);
        state.set_phase(MAINTENANCE_CLEAN);
    }
    crate::app::startup_state::resume_startup_after_maintenance_run(app);
}

pub(crate) fn begin_recovery_replay<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    begin_maintenance(app);
}

pub(crate) fn finish_recovery_replay<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    finish_maintenance(app);
}

pub(crate) fn finish_recovery_replay_for_startup<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    finish_maintenance_for_startup(app);
}

pub(crate) fn fail_recovery_replay<R: tauri::Runtime>(app: &tauri::AppHandle<R>, error: AppError) {
    if let Some(state) = app.try_state::<MaintenanceState>() {
        state.set_phase(MAINTENANCE_FAILED);
    }
    if app
        .try_state::<crate::app::startup_state::StartupState>()
        .is_some()
    {
        crate::app::startup_state::fail_maintenance_run_at_stage(
            app,
            crate::app::startup_state::AppStartupStage::InitializingDb,
            recovery_failure_message(&error),
        );
    }
}

/// Consume a pending reset synchronously.  This is called from Tauri setup,
/// before logging, DB initialization, or any normal background owner exists.
pub(crate) fn run_before_startup<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    let Some(state) = app.try_state::<MaintenanceState>() else {
        return true;
    };
    let _lock = state.lock_coordinator();

    let data_dir = match crate::app_paths::app_data_dir(app) {
        Ok(path) => path,
        Err(error) => {
            fail_maintenance(app, error);
            return false;
        }
    };
    let marker_state = match reset_marker_state(&data_dir) {
        Ok(value) => value,
        Err(error) => {
            fail_maintenance(app, error);
            return false;
        }
    };
    if marker_state == ResetMarkerState::None {
        finish_maintenance(app);
        return true;
    }

    begin_maintenance(app);
    let db_path = match crate::db::db_path(app) {
        Ok(path) => path,
        Err(error) => {
            fail_maintenance(app, error);
            return false;
        }
    };
    match consume_reset_marker_at(&data_dir, &db_path) {
        Ok(true) => {
            finish_maintenance(app);
            true
        }
        Ok(false) => {
            finish_maintenance(app);
            true
        }
        Err(error) => {
            fail_maintenance(app, error);
            false
        }
    }
}

pub(crate) async fn retry_pending_maintenance<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> bool {
    let Some(state) = app.try_state::<MaintenanceState>() else {
        return false;
    };
    if !state.try_begin_retry() {
        return false;
    }
    drop(state);
    crate::app::startup_state::begin_maintenance_run(&app);

    let marker_state =
        crate::app_paths::app_data_dir(&app).and_then(|data_dir| reset_marker_state(&data_dir));
    match marker_state {
        Ok(ResetMarkerState::Pending | ResetMarkerState::Completed) => {
            let app_for_work = app.clone();
            match crate::blocking::run("maintenance_reset_retry", move || {
                Ok::<_, AppError>(run_before_startup(&app_for_work))
            })
            .await
            {
                Ok(result) => result,
                Err(error) => {
                    fail_maintenance(&app, error);
                    false
                }
            }
        }
        Err(error) => {
            fail_maintenance(&app, error);
            false
        }
        Ok(ResetMarkerState::None) => retry_recovery_journal(app).await,
    }
}

async fn retry_recovery_journal<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> bool {
    let db = {
        let state = app.state::<crate::app::app_state::DbInitState>();
        let db = state.0.lock().await.clone();
        db
    };
    let db = match db {
        Some(db) => db,
        None => {
            let state = app.state::<crate::app::app_state::DbInitState>();
            match crate::app::app_state::ensure_db_ready_for_recovery(app.clone(), state.inner())
                .await
            {
                Ok(db) => db,
                Err(error) => {
                    fail_recovery_replay(&app, error);
                    return false;
                }
            }
        }
    };

    let app_for_work = app.clone();
    let db_for_work = db.clone();
    match crate::blocking::run("recovery_journal_retry", move || {
        crate::infra::recovery_journal::replay_pending(&app_for_work, &db_for_work)
    })
    .await
    {
        Ok(_) => {
            finish_recovery_replay(&app);
            true
        }
        Err(error) => {
            fail_recovery_replay(&app, error);
            false
        }
    }
}

pub(crate) fn request_reset_and_exit<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> AppResult<bool> {
    let Some(state) = app.try_state::<MaintenanceState>() else {
        return Err(AppError::new(
            "APP_MAINTENANCE_UNAVAILABLE",
            "应用维护状态不可用",
        ));
    };
    let _lock = state.lock_coordinator();
    let data_dir = crate::app_paths::app_data_dir(&app)?;
    state.set_phase(MAINTENANCE_RUNNING);
    if let Err(error) = write_reset_marker_at(&data_dir) {
        state.set_phase(MAINTENANCE_CLEAN);
        return Err(error);
    }
    state.request_reset_exit();
    crate::app::startup_state::begin_maintenance_run(&app);
    drop(_lock);

    if let Some(state) = app.try_state::<crate::app::resident::ResidentState>() {
        state.begin_exit();
    }
    // Do not return to the running process after the durable marker exists.
    // A hard exit is intentional here: normal Tauri cleanup can reconcile or
    // reopen SQLite, and detached runtime owners cannot be atomically drained.
    std::process::exit(0)
}

pub(crate) fn should_skip_exit_cleanup<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    app.try_state::<MaintenanceState>()
        .is_some_and(|state| state.should_skip_exit_cleanup())
}

pub(crate) fn start_normal_runtime_once(app: &tauri::AppHandle) -> bool {
    let Some(state) = app.try_state::<MaintenanceState>() else {
        return true;
    };
    state.try_mark_runtime_started()
}

pub(crate) fn normal_runtime_started<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    app.try_state::<MaintenanceState>()
        .is_some_and(|state| state.runtime_started.load(Ordering::Acquire))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_write_is_idempotent_and_validates_existing_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(write_reset_marker_at(dir.path()).expect("write marker"));
        assert!(!write_reset_marker_at(dir.path()).expect("repeat marker"));
        assert_eq!(
            std::fs::read(marker_path_for_data_dir(dir.path())).expect("read marker"),
            RESET_MARKER_CONTENT
        );
    }

    #[test]
    fn malformed_marker_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker = marker_path_for_data_dir(dir.path());
        std::fs::create_dir_all(marker.parent().expect("marker parent")).expect("mkdir");
        std::fs::write(&marker, b"not-a-reset").expect("write malformed marker");
        let error = write_reset_marker_at(dir.path()).expect_err("malformed marker must fail");
        assert_eq!(error.code(), "APP_MAINTENANCE_MARKER_INVALID");
    }

    #[test]
    fn marker_survives_failed_consumption_and_can_be_retried() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_reset_marker_at(dir.path()).expect("write marker");
        let db_path = dir.path().join("blocked.db");
        std::fs::create_dir_all(&db_path).expect("create blocking directory");
        let error = consume_reset_marker_at(dir.path(), &db_path).expect_err("reset fails");
        assert_eq!(error.code(), "APP_DATA_RESET_INCOMPLETE");
        assert!(marker_path_for_data_dir(dir.path()).exists());

        std::fs::remove_dir(&db_path).expect("remove blocking directory");
        std::fs::write(dir.path().join("settings.json"), b"stale settings")
            .expect("write reset target");
        assert!(consume_reset_marker_at(dir.path(), &db_path).expect("retry reset"));
        assert!(!marker_path_for_data_dir(dir.path()).exists());
        assert!(!dir.path().join("settings.json").exists());
    }

    #[test]
    fn maintenance_invoke_gate_only_allows_status_retry_and_exit() {
        let state = MaintenanceState::default();
        assert!(state.allows_invoke("provider_upsert"));

        state.set_phase(MAINTENANCE_FAILED);
        assert!(state.allows_invoke("app_startup_status_get"));
        assert!(state.allows_invoke("app_startup_retry"));
        assert!(state.allows_invoke("app_exit"));
        assert!(!state.allows_invoke("cli_proxy_sync_enabled"));
        assert!(!state.allows_invoke("provider_upsert"));

        state.request_reset_exit();
        assert!(!state.allows_invoke("app_startup_retry"));
        assert!(state.should_skip_exit_cleanup());
    }

    #[test]
    fn maintenance_retry_claim_is_single_flight() {
        let state = MaintenanceState::default();
        state.set_phase(MAINTENANCE_FAILED);

        assert!(state.try_begin_retry());
        assert!(!state.try_begin_retry());
        assert_eq!(state.phase(), MAINTENANCE_RUNNING);
    }
}
