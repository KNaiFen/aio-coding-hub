//! Usage: App data and DB disk-management helpers (reset, usage stats, cleanup).

use crate::db;
use crate::shared::error::db_err;
use crate::usage_ledger;
use rusqlite::TransactionBehavior;
use serde::Serialize;
use std::io;
use std::path::{Path, PathBuf};

const USAGE_LEDGER_COVERAGE_INCOMPLETE_ERROR_CODE: &str = "USAGE_LEDGER_COVERAGE_INCOMPLETE";

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DbDiskUsage {
    pub db_bytes: u64,
    pub wal_bytes: u64,
    pub shm_bytes: u64,
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClearRequestLogsResult {
    pub request_logs_deleted: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DbCompactResult {
    pub before_bytes: u64,
    pub after_bytes: u64,
}

fn file_len_or_zero(path: &Path) -> Result<u64, String> {
    match std::fs::metadata(path) {
        Ok(meta) => Ok(meta.len()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(err) => Err(format!("failed to stat {}: {err}", path.to_string_lossy())),
    }
}

fn remove_file_if_exists(path: &Path) -> Result<bool, std::io::Error> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

fn db_related_paths(db_path: &Path) -> (PathBuf, PathBuf) {
    let wal_path = {
        let mut out = db_path.to_path_buf().into_os_string();
        out.push("-wal");
        PathBuf::from(out)
    };
    let shm_path = {
        let mut out = db_path.to_path_buf().into_os_string();
        out.push("-shm");
        PathBuf::from(out)
    };
    (wal_path, shm_path)
}

fn sqlite_reclaimable_bytes(db: &db::Db) -> crate::shared::error::AppResult<u64> {
    let conn = db.open_connection()?;
    let freelist_count: i64 = conn
        .query_row("PRAGMA freelist_count", [], |row| row.get(0))
        .map_err(|e| db_err!("failed to read SQLite freelist_count: {e}"))?;
    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .map_err(|e| db_err!("failed to read SQLite page_size: {e}"))?;
    if freelist_count < 0 || page_size < 0 {
        return Err(db_err!(
            "SQLite space metrics must be non-negative: freelist_count={freelist_count}, page_size={page_size}"
        ));
    }

    Ok((freelist_count as u64).saturating_mul(page_size as u64))
}

fn disk_usage_at(
    db_path: &Path,
    db: &db::Db,
) -> crate::shared::error::AppResult<DbDiskUsage> {
    let (wal_path, shm_path) = db_related_paths(db_path);

    let db_bytes = file_len_or_zero(db_path)?;
    let wal_bytes = file_len_or_zero(&wal_path)?;
    let shm_bytes = file_len_or_zero(&shm_path)?;

    Ok(DbDiskUsage {
        db_bytes,
        wal_bytes,
        shm_bytes,
        total_bytes: db_bytes.saturating_add(wal_bytes).saturating_add(shm_bytes),
        reclaimable_bytes: sqlite_reclaimable_bytes(db)?,
    })
}

pub fn db_disk_usage_get<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
) -> crate::shared::error::AppResult<DbDiskUsage> {
    let db_path = db::db_path(app)?;
    disk_usage_at(&db_path, db)
}

pub fn db_compact(
    app: &tauri::AppHandle,
    db: &db::Db,
) -> crate::shared::error::AppResult<DbCompactResult> {
    let db_path = db::db_path(app)?;
    db_compact_at(&db_path, db)
}

fn db_compact_at(db_path: &Path, db: &db::Db) -> crate::shared::error::AppResult<DbCompactResult> {
    tracing::info!("compacting database (user-initiated)");

    let before_bytes = disk_usage_at(db_path, db)?.total_bytes;

    let conn = db.open_connection()?;

    // Checkpoints stay best-effort, but VACUUM failures must surface because
    // this command is the user's explicit request to return reusable pages.
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    conn.execute_batch("VACUUM;")
        .map_err(|e| db_err!("failed to vacuum database: {e}"))?;
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    let after_bytes = disk_usage_at(db_path, db)?.total_bytes;

    tracing::info!(before_bytes, after_bytes, "database compacted");

    Ok(DbCompactResult {
        before_bytes,
        after_bytes,
    })
}

fn request_logs_have_usage_ledger_coverage(conn: &rusqlite::Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        r#"
SELECT NOT EXISTS (
  SELECT 1
  FROM request_logs request
  WHERE NOT EXISTS (
    SELECT 1
    FROM usage_ledger ledger
    WHERE ledger.request_log_id = request.id
      AND ledger.trace_id = request.trace_id
  )
)
"#,
        [],
        |row| row.get(0),
    )
}

pub fn request_logs_clear_all(
    db: &db::Db,
) -> crate::shared::error::AppResult<ClearRequestLogsResult> {
    tracing::warn!("clearing all request logs (user-initiated)");

    let mut conn = db.open_connection()?;

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let backfill_complete = usage_ledger::is_backfill_complete(&tx)
        .map_err(|e| db_err!("failed to read usage ledger backfill state: {e}"))?;
    if !backfill_complete {
        return Err(crate::shared::error::AppError::new(
            usage_ledger::USAGE_LEDGER_BACKFILL_INCOMPLETE_ERROR_CODE,
            "usage ledger backfill is incomplete; request logs were not cleared",
        ));
    }
    let has_full_coverage = request_logs_have_usage_ledger_coverage(&tx)
        .map_err(|e| db_err!("failed to verify usage ledger coverage: {e}"))?;
    if !has_full_coverage {
        return Err(crate::shared::error::AppError::new(
            USAGE_LEDGER_COVERAGE_INCOMPLETE_ERROR_CODE,
            "usage ledger coverage is incomplete; request logs were not cleared",
        ));
    }

    let request_logs_deleted = tx
        .execute("DELETE FROM request_logs", [])
        .map_err(|e| db_err!("failed to clear request_logs: {e}"))?;

    tx.commit()
        .map_err(|e| db_err!("failed to commit transaction: {e}"))?;

    tracing::warn!(
        request_logs_deleted = request_logs_deleted,
        "request logs cleared"
    );

    // Keep reusable pages visible to the user. Returning them to the filesystem
    // is reserved for the explicit database-compaction command.
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    Ok(ClearRequestLogsResult {
        request_logs_deleted: request_logs_deleted as u64,
    })
}

/// Delete the complete reset target set.  The caller owns the durable marker;
/// this helper is intentionally idempotent so a later retry can finish a
/// partially completed process without reopening the database.
pub(crate) fn app_data_reset_at(
    dir: &Path,
    db_path: &Path,
) -> crate::shared::error::AppResult<bool> {
    let (wal_path, shm_path) = db_related_paths(db_path);
    let targets = [
        ("settings_tmp", dir.join("settings.json.tmp")),
        ("settings_backup", dir.join("settings.json.bak")),
        ("settings", dir.join("settings.json")),
        ("sqlite_wal", wal_path),
        ("sqlite_shm", shm_path),
        ("sqlite", db_path.to_path_buf()),
    ];
    let mut failed = Vec::new();
    for (label, path) in targets {
        if remove_file_if_exists(&path).is_err() {
            failed.push(label);
        }
    }

    if !failed.is_empty() {
        return Err(crate::shared::error::AppError::new(
            "APP_DATA_RESET_INCOMPLETE",
            format!("failed reset targets: {}", failed.join(", ")),
        ));
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{
        db_compact_at, disk_usage_at, request_logs_clear_all,
        USAGE_LEDGER_COVERAGE_INCOMPLETE_ERROR_CODE,
    };
    use rusqlite::params;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn init_test_db() -> (crate::db::Db, PathBuf, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("data-management.sqlite");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        (db, db_path, dir)
    }

    fn insert_request_log_rows(db: &crate::db::Db, count: usize) {
        let conn = db.open_connection().expect("open connection");
        // Bulky payload so deletions leave measurable free pages behind.
        let attempts_json = format!("[\"{}\"]", "x".repeat(4096));
        for idx in 0..count {
            conn.execute(
                r#"
INSERT INTO request_logs (
  trace_id, cli_key, method, path, status, duration_ms, attempts_json,
  created_at, created_at_ms, excluded_from_stats
) VALUES (?1, 'claude', 'POST', '/v1/messages', 200, 10, ?2, 1770000000, 1770000000000, 0)
"#,
                params![format!("trace-compact-{idx}"), attempts_json],
            )
            .expect("insert request log row");
        }
    }

    fn count_request_logs(db: &crate::db::Db) -> i64 {
        let conn = db.open_connection().expect("open connection");
        conn.query_row("SELECT COUNT(1) FROM request_logs", [], |row| row.get(0))
            .expect("count request logs")
    }

    fn count_usage_ledger(db: &crate::db::Db) -> i64 {
        let conn = db.open_connection().expect("open connection");
        conn.query_row("SELECT COUNT(1) FROM usage_ledger", [], |row| row.get(0))
            .expect("count usage ledger")
    }

    fn project_request_log_to_ledger(db: &crate::db::Db, trace_id: &str) {
        let conn = db.open_connection().expect("open connection");
        assert_eq!(
            crate::usage_ledger::project_trace(&conn, trace_id)
                .expect("project request log to usage ledger"),
            1
        );
    }

    fn project_all_request_logs_to_ledger(db: &crate::db::Db) {
        let trace_ids = {
            let conn = db.open_connection().expect("open connection");
            let mut statement = conn
                .prepare("SELECT trace_id FROM request_logs ORDER BY id ASC")
                .expect("prepare trace query");
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .expect("query trace ids");
            rows
                .collect::<Result<Vec<_>, _>>()
                .expect("collect trace ids")
        };
        for trace_id in trace_ids {
            project_request_log_to_ledger(db, &trace_id);
        }
    }

    #[test]
    fn db_compact_keeps_rows_and_reclaims_space() {
        let (db, db_path, _dir) = init_test_db();

        insert_request_log_rows(&db, 300);
        {
            let conn = db.open_connection().expect("open connection");
            conn.execute("DELETE FROM request_logs WHERE rowid % 4 != 0", [])
                .expect("delete rows");
        }
        let rows_before = count_request_logs(&db);
        assert!(rows_before > 0, "expected surviving rows before compact");
        let reclaimable_before = disk_usage_at(&db_path, &db)
            .expect("read usage before compact")
            .reclaimable_bytes;
        assert!(
            reclaimable_before > 0,
            "expected reusable pages before compact"
        );

        let result = db_compact_at(&db_path, &db).expect("compact db");
        let reclaimable_after = disk_usage_at(&db_path, &db)
            .expect("read usage after compact")
            .reclaimable_bytes;

        assert_eq!(
            count_request_logs(&db),
            rows_before,
            "compact must not delete data"
        );
        assert!(
            result.after_bytes <= result.before_bytes,
            "after_bytes {} must not exceed before_bytes {}",
            result.after_bytes,
            result.before_bytes
        );
        assert!(
            reclaimable_after < reclaimable_before,
            "manual compact should reduce reusable pages"
        );
    }

    #[test]
    fn db_compact_surfaces_vacuum_failure_and_keeps_rows() {
        let (db, db_path, _dir) = init_test_db();
        insert_request_log_rows(&db, 4);

        // Hold the write lock on a separate connection so VACUUM cannot acquire it.
        let blocker = rusqlite::Connection::open(&db_path).expect("open blocker connection");
        blocker
            .execute_batch("BEGIN IMMEDIATE;")
            .expect("begin immediate");

        let err = db_compact_at(&db_path, &db).expect_err("vacuum must fail while db is locked");
        assert!(
            err.to_string().contains("failed to vacuum database"),
            "unexpected error: {err}"
        );

        blocker.execute_batch("ROLLBACK;").expect("rollback");
        assert_eq!(
            count_request_logs(&db),
            4,
            "rows must survive failed compact"
        );
    }

    #[test]
    fn request_logs_clear_all_rejects_incomplete_backfill_without_deleting() {
        let (db, _db_path, _dir) = init_test_db();
        insert_request_log_rows(&db, 1);
        {
            let conn = db.open_connection().expect("open connection");
            conn.execute(
                r#"
UPDATE usage_ledger_backfill_state
SET
  status = 'incomplete',
  target_request_log_id = (SELECT MAX(id) FROM request_logs),
  last_request_log_id = 0,
  completed_at = NULL
WHERE id = 1
"#,
                [],
            )
            .expect("mark usage ledger backfill incomplete");
        }

        let error = request_logs_clear_all(&db).expect_err("incomplete backfill must block clear");
        assert_eq!(
            error.code(),
            crate::usage_ledger::USAGE_LEDGER_BACKFILL_INCOMPLETE_ERROR_CODE
        );
        assert_eq!(count_request_logs(&db), 1);
        assert_eq!(count_usage_ledger(&db), 0);
    }

    #[test]
    fn request_logs_clear_all_rejects_missing_ledger_coverage_without_deleting() {
        let (db, _db_path, _dir) = init_test_db();
        insert_request_log_rows(&db, 1);

        let error =
            request_logs_clear_all(&db).expect_err("missing ledger coverage must block clear");
        assert_eq!(error.code(), USAGE_LEDGER_COVERAGE_INCOMPLETE_ERROR_CODE);
        assert_eq!(count_request_logs(&db), 1);
        assert_eq!(count_usage_ledger(&db), 0);
    }

    #[test]
    fn request_logs_clear_all_preserves_usage_ledger_after_backfill() {
        let (db, _db_path, _dir) = init_test_db();
        insert_request_log_rows(&db, 1);
        project_request_log_to_ledger(&db, "trace-compact-0");

        let result = request_logs_clear_all(&db).expect("clear request logs");

        assert_eq!(result.request_logs_deleted, 1);
        assert_eq!(count_request_logs(&db), 0);
        assert_eq!(count_usage_ledger(&db), 1);
    }

    #[test]
    fn request_logs_clear_all_leaves_reclaimable_pages_for_manual_compaction() {
        let (db, db_path, _dir) = init_test_db();
        insert_request_log_rows(&db, 300);
        project_all_request_logs_to_ledger(&db);
        let before = disk_usage_at(&db_path, &db).expect("usage before clear");

        let result = request_logs_clear_all(&db).expect("clear request logs");
        let after = disk_usage_at(&db_path, &db).expect("usage after clear");

        assert_eq!(result.request_logs_deleted, 300);
        assert_eq!(count_request_logs(&db), 0);
        assert_eq!(count_usage_ledger(&db), 300);
        assert!(
            after.reclaimable_bytes > before.reclaimable_bytes,
            "clear should expose reusable SQLite pages without vacuuming"
        );
    }
}
