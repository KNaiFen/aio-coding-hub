//! Usage: SQLite migration v49->v50 - external side-effect recovery journal.
//!
//! The base journal stores bounded metadata only. Content recovery artifacts,
//! when required, live below the app-owned recovery root and are referenced by
//! opaque relative identifiers rather than arbitrary filesystem paths.

use rusqlite::Connection;

pub(super) fn create_recovery_journal_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS external_effect_recovery_journal (
  operation_id TEXT PRIMARY KEY,
  parent_operation_id TEXT,
  operation_kind TEXT NOT NULL,
  cli_key TEXT,
  workspace_id INTEGER,
  entity_id INTEGER,
  phase TEXT NOT NULL DEFAULT 'prepare',
  status TEXT NOT NULL DEFAULT 'prepared',
  artifact_ref TEXT,
  artifact_sha256 TEXT,
  attempt_count INTEGER NOT NULL DEFAULT 0,
  error_code TEXT,
  error_summary TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  CHECK(status IN ('prepared', 'projecting', 'committed', 'resolved', 'failed')),
  CHECK(length(operation_id) BETWEEN 1 AND 64),
  CHECK(length(operation_kind) BETWEEN 1 AND 96),
  CHECK(cli_key IS NULL OR length(cli_key) BETWEEN 1 AND 32),
  CHECK(artifact_ref IS NULL OR (length(artifact_ref) BETWEEN 1 AND 512 AND artifact_ref NOT LIKE '/%' AND artifact_ref NOT LIKE '%..%')),
  CHECK(artifact_sha256 IS NULL OR length(artifact_sha256) = 64),
  CHECK(error_code IS NULL OR length(error_code) BETWEEN 1 AND 96),
  CHECK(error_summary IS NULL OR length(error_summary) <= 512)
);

CREATE INDEX IF NOT EXISTS idx_external_effect_recovery_pending
  ON external_effect_recovery_journal(status, created_at, operation_id);
CREATE INDEX IF NOT EXISTS idx_external_effect_recovery_parent
  ON external_effect_recovery_journal(parent_operation_id, created_at);
"#,
    )
    .map_err(|error| format!("failed to create external effect recovery journal: {error}"))
}

pub(super) fn migrate_v49_to_v50(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v49->v50 transaction: {error}"))?;
    create_recovery_journal_schema(&tx)?;
    super::set_user_version(&tx, 50)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v49->v50 transaction: {error}"))?;
    Ok(())
}
