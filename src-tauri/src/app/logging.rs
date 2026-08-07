//! Usage: Tracing/logging initialization (rolling file logs + best-effort cleanup).

use crate::{app_paths, blocking, settings};
use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;

const LOG_SUBDIR: &str = "logs";
const LOG_FILE_PREFIX: &str = "aio-coding-hub.log";
const LOG_FILE_DATE_FORMAT: &str = "%Y-%m-%d";
const LOG_SOFT_LIMIT_BYTES: u64 = 256 * 1024 * 1024;
const CLEANUP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

static TRACING_GUARD: OnceLock<Mutex<Option<WorkerGuard>>> = OnceLock::new();
static TRACING_INIT: OnceLock<()> = OnceLock::new();

pub(crate) fn init(app: &tauri::AppHandle) {
    TRACING_INIT.get_or_init(|| {
        let app = app.clone();
        if let Err(err) = init_impl(&app) {
            // Last-resort fallback: stderr logger (may be invisible on Windows release).
            let _ = tracing_subscriber::fmt()
                .with_env_filter(default_env_filter())
                .with_target(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .try_init();
            eprintln!("tracing init failed: {err}");
        }
    });
}

fn init_impl(app: &tauri::AppHandle) -> crate::shared::error::AppResult<()> {
    let log_dir = ensure_log_dir(app)?;
    let env_filter = default_env_filter();

    let file_appender = tracing_appender::rolling::daily(&log_dir, LOG_FILE_PREFIX);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    TRACING_GUARD
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "logging guard mutex poisoned".to_string())?
        .replace(guard);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    #[cfg(debug_assertions)]
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer);

    #[cfg(debug_assertions)]
    let subscriber = subscriber.with(stdout_layer);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| format!("failed to set global tracing subscriber: {e}"))?;

    // Capture `log` crate records (from dependencies) into `tracing` when possible.
    // If another logger is already set (e.g. by a dependency), skip silently.
    let _ = tracing_log::LogTracer::init();

    tracing::info!(log_dir = %log_dir.display(), "tracing initialized");

    spawn_cleanup_task(app.clone(), log_dir);

    Ok(())
}

fn default_env_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        #[cfg(debug_assertions)]
        {
            tracing_subscriber::EnvFilter::new("info,aio_coding_hub_lib=debug,aio_coding_hub=debug")
        }
        #[cfg(not(debug_assertions))]
        {
            tracing_subscriber::EnvFilter::new("info")
        }
    })
}

fn ensure_log_dir(app: &tauri::AppHandle) -> crate::shared::error::AppResult<PathBuf> {
    let base = app_paths::app_data_dir(app)?;
    let dir = base.join(LOG_SUBDIR);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create log dir {}: {e}", dir.display()))?;
    Ok(dir)
}

fn spawn_cleanup_task(app: tauri::AppHandle, log_dir: PathBuf) {
    tauri::async_runtime::spawn(async move {
        run_cleanup_once_blocking(app.clone(), log_dir.clone()).await;

        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // First tick is immediate; skip it so we don't run twice at startup.
        interval.tick().await;
        loop {
            interval.tick().await;
            run_cleanup_once_blocking(app.clone(), log_dir.clone()).await;
        }
    });
}

async fn run_cleanup_once_blocking(app: tauri::AppHandle, log_dir: PathBuf) {
    if let Err(err) = blocking::run("log_cleanup", move || {
        cleanup_once(&app, &log_dir);
        Ok::<(), String>(())
    })
    .await
    {
        tracing::warn!("log cleanup task failed: {}", err);
    }
}

fn cleanup_once(app: &tauri::AppHandle, log_dir: &Path) {
    let retention_days = settings::log_retention_days_fail_open(app).max(1);
    match cleanup_logs(log_dir, retention_days) {
        Ok(report) => {
            if report.deleted_files > 0 {
                tracing::info!(
                    retention_days,
                    deleted_files = report.deleted_files,
                    deleted_bytes = report.deleted_bytes,
                    remaining_bytes = report.remaining_bytes,
                    "cleaned up closed rolling log files"
                );
            }
            if report.remaining_bytes > LOG_SOFT_LIMIT_BYTES {
                tracing::warn!(
                    soft_limit_bytes = LOG_SOFT_LIMIT_BYTES,
                    remaining_bytes = report.remaining_bytes,
                    closed_files_remaining = report.closed_files_remaining,
                    "log soft limit remains exceeded; protected or undeletable files were retained"
                );
            }
        }
        Err(err) => {
            tracing::warn!(retention_days, "log cleanup failed: {}", err);
        }
    }
}

#[derive(Debug, Clone)]
struct RollingLogFile {
    path: PathBuf,
    date: NaiveDate,
    bytes: u64,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct LogCleanupReport {
    deleted_files: usize,
    deleted_bytes: u64,
    remaining_bytes: u64,
    closed_files_remaining: usize,
}

fn parse_rolling_log_date(name: &str) -> Option<NaiveDate> {
    let suffix = name.strip_prefix(LOG_FILE_PREFIX)?.strip_prefix('.')?;
    if suffix.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(suffix, LOG_FILE_DATE_FORMAT).ok()
}

fn collect_rolling_logs(log_dir: &Path) -> crate::shared::error::AppResult<Vec<RollingLogFile>> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(log_dir).map_err(|e| format!("read_dir failed: {e}"))?;
    for entry in entries {
        let entry = match entry {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!("log cleanup: read_dir entry error: {}", err);
                continue;
            }
        };

        let path = entry.path();
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        let Some(date) = parse_rolling_log_date(name) else {
            continue;
        };
        let file_type = match entry.file_type() {
            Ok(value) => value,
            Err(err) => {
                tracing::warn!(path = %path.display(), "log cleanup: file type error: {}", err);
                continue;
            }
        };
        if !file_type.is_file() {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(path = %path.display(), "log cleanup: metadata error: {}", err);
                continue;
            }
        };
        files.push(RollingLogFile {
            path,
            date,
            bytes: meta.len(),
        });
    }
    files.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(files)
}

fn remove_rolling_log(file: &RollingLogFile, report: &mut LogCleanupReport) -> bool {
    match std::fs::remove_file(&file.path) {
        Ok(()) => {
            report.deleted_files = report.deleted_files.saturating_add(1);
            report.deleted_bytes = report.deleted_bytes.saturating_add(file.bytes);
            report.remaining_bytes = report.remaining_bytes.saturating_sub(file.bytes);
            true
        }
        Err(err) => {
            tracing::warn!(path = %file.path.display(), "log cleanup: remove failed: {}", err);
            false
        }
    }
}

fn cleanup_logs(
    log_dir: &Path,
    retention_days: u32,
) -> crate::shared::error::AppResult<LogCleanupReport> {
    let today = DateTime::<Utc>::from(SystemTime::now()).date_naive();
    cleanup_logs_at(log_dir, retention_days, LOG_SOFT_LIMIT_BYTES, today)
}

fn cleanup_logs_at(
    log_dir: &Path,
    retention_days: u32,
    soft_limit_bytes: u64,
    today: NaiveDate,
) -> crate::shared::error::AppResult<LogCleanupReport> {
    let retention_days = retention_days.max(1);
    let keep_from = today
        .checked_sub_signed(ChronoDuration::days(i64::from(retention_days - 1)))
        .unwrap_or(NaiveDate::MIN);
    let files = collect_rolling_logs(log_dir)?;
    // Rotation happens lazily on write. Around UTC midnight the currently open
    // file can still carry yesterday's suffix, so protect the latest dated file
    // at or before today in addition to all current/future dates.
    let active_path = files
        .iter()
        .filter(|file| file.date <= today)
        .max_by(|left, right| {
            left.date
                .cmp(&right.date)
                .then_with(|| left.path.cmp(&right.path))
        })
        .map(|file| file.path.clone());
    let mut report = LogCleanupReport {
        remaining_bytes: files
            .iter()
            .fold(0u64, |total, file| total.saturating_add(file.bytes)),
        ..Default::default()
    };
    let mut retained = Vec::new();
    let mut failed_removals = HashSet::new();

    for file in files {
        if file.date < keep_from && active_path.as_ref() != Some(&file.path) {
            if !remove_rolling_log(&file, &mut report) {
                failed_removals.insert(file.path.clone());
                retained.push(file);
            }
        } else {
            retained.push(file);
        }
    }

    let mut capacity_deleted = HashSet::new();
    if report.remaining_bytes > soft_limit_bytes {
        for file in &retained {
            if report.remaining_bytes <= soft_limit_bytes {
                break;
            }
            if file.date >= today
                || active_path.as_ref() == Some(&file.path)
                || failed_removals.contains(&file.path)
            {
                continue;
            }
            if remove_rolling_log(file, &mut report) {
                capacity_deleted.insert(file.path.clone());
            } else {
                failed_removals.insert(file.path.clone());
            }
        }
    }

    report.closed_files_remaining = retained
        .iter()
        .filter(|file| {
            file.date < today
                && active_path.as_ref() != Some(&file.path)
                && !capacity_deleted.contains(&file.path)
        })
        .count();

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_log_parser_accepts_only_exact_daily_names() {
        assert_eq!(
            parse_rolling_log_date("aio-coding-hub.log.2026-08-07"),
            NaiveDate::from_ymd_opt(2026, 8, 7)
        );
        for name in [
            "aio-coding-hub.log",
            "aio-coding-hub.log.2026-8-07",
            "aio-coding-hub.log.2026-08-07.tmp",
            "aio-coding-hub.log.2026-02-30",
            "aio-coding-hub.log.directory",
            "other.log.2026-08-07",
        ] {
            assert_eq!(
                parse_rolling_log_date(name),
                None,
                "unexpected match: {name}"
            );
        }
    }

    #[test]
    fn cleanup_logs_deletes_only_expired_closed_daily_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let expired_log = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-07-31"));
        let retained_log = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-08-01"));
        let active_log = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-08-07"));
        let unrelated = dir.path().join("other.log");
        let matching_dir = dir.path().join(format!("{LOG_FILE_PREFIX}.directory"));
        let exact_name_dir = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-07-30"));
        std::fs::write(&expired_log, "expired log").expect("write expired log");
        std::fs::write(&retained_log, "retained log").expect("write retained log");
        std::fs::write(&active_log, "active log").expect("write active log");
        std::fs::write(&unrelated, "keep me").expect("write unrelated file");
        std::fs::create_dir(&matching_dir).expect("create matching dir");
        std::fs::create_dir(&exact_name_dir).expect("create exact-name directory");

        let report = cleanup_logs_at(
            dir.path(),
            7,
            u64::MAX,
            NaiveDate::from_ymd_opt(2026, 8, 7).expect("valid date"),
        )
        .expect("cleanup succeeds");

        assert_eq!(report.deleted_files, 1);
        assert!(!expired_log.exists());
        assert!(retained_log.exists());
        assert!(active_log.exists());
        assert!(unrelated.exists());
        assert!(matching_dir.exists());
        assert!(exact_name_dir.exists());
    }

    #[test]
    fn cleanup_logs_enforces_soft_limit_from_oldest_closed_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let oldest = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-08-04"));
        let newer = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-08-05"));
        let active = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-08-07"));
        std::fs::write(&oldest, [0u8; 6]).expect("write oldest log");
        std::fs::write(&newer, [0u8; 6]).expect("write newer log");
        std::fs::write(&active, [0u8; 6]).expect("write active log");

        let report = cleanup_logs_at(
            dir.path(),
            7,
            13,
            NaiveDate::from_ymd_opt(2026, 8, 7).expect("valid date"),
        )
        .expect("cleanup succeeds");

        assert_eq!(report.deleted_files, 1);
        assert_eq!(report.remaining_bytes, 12);
        assert!(!oldest.exists());
        assert!(newer.exists());
        assert!(active.exists());
    }

    #[test]
    fn cleanup_logs_preserves_active_file_over_soft_limit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let active = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-08-07"));
        std::fs::write(&active, [0u8; 16]).expect("write active log");

        let report = cleanup_logs_at(
            dir.path(),
            7,
            10,
            NaiveDate::from_ymd_opt(2026, 8, 7).expect("valid date"),
        )
        .expect("cleanup succeeds");

        assert_eq!(report.deleted_files, 0);
        assert_eq!(report.remaining_bytes, 16);
        assert_eq!(report.closed_files_remaining, 0);
        assert!(active.exists());
    }

    #[test]
    fn cleanup_logs_protects_latest_file_before_lazy_rollover() {
        let dir = tempfile::tempdir().expect("tempdir");
        let closed = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-08-05"));
        let still_active = dir.path().join(format!("{LOG_FILE_PREFIX}.2026-08-06"));
        std::fs::write(&closed, [0u8; 6]).expect("write closed log");
        std::fs::write(&still_active, [0u8; 6]).expect("write active log");

        let report = cleanup_logs_at(
            dir.path(),
            1,
            u64::MAX,
            NaiveDate::from_ymd_opt(2026, 8, 7).expect("valid date"),
        )
        .expect("cleanup succeeds");

        assert_eq!(report.deleted_files, 1);
        assert!(!closed.exists());
        assert!(still_active.exists());
    }
}
