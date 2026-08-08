//! Usage: SQLite migration v50->v51 - recovery journal fencing and replay context.

use rusqlite::{Connection, OptionalExtension};

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2 LIMIT 1",
        (table, column),
        |_row| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(|error| format!("failed to inspect {table}.{column}: {error}"))
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    if column_exists(conn, table, column)? {
        return Ok(());
    }
    conn.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition};"
    ))
    .map_err(|error| format!("failed to add {table}.{column}: {error}"))
}

pub(super) fn create_recovery_claim_schema(conn: &Connection) -> Result<(), String> {
    super::v49_to_v50::create_recovery_journal_schema(conn)?;
    add_column_if_missing(
        conn,
        "external_effect_recovery_journal",
        "lease_owner",
        "TEXT",
    )?;
    add_column_if_missing(
        conn,
        "external_effect_recovery_journal",
        "lease_expires_at",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "external_effect_recovery_journal",
        "claim_epoch",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "external_effect_recovery_journal",
        "replay_context",
        "TEXT",
    )?;

    // Older candidates temporarily overloaded `phase` with the replay state.
    // Restore the durable domain checkpoint before the fenced claim schema is
    // used; otherwise workspace replay cannot interpret those rows.
    conn.execute(
        "UPDATE external_effect_recovery_journal SET phase = 'prepare' WHERE phase = 'startup_replay' AND status != 'resolved'",
        [],
    )
    .map_err(|error| format!("failed to normalize legacy replay phase: {error}"))?;

    conn.execute_batch(
        r#"
CREATE INDEX IF NOT EXISTS idx_external_effect_recovery_claimable
  ON external_effect_recovery_journal(status, lease_expires_at, created_at, operation_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_external_effect_recovery_parent_phase
  ON external_effect_recovery_journal(parent_operation_id, phase)
  WHERE parent_operation_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_external_effect_recovery_workspace_apply_cli
  ON external_effect_recovery_journal(cli_key)
  WHERE operation_kind = 'workspace.apply' AND status != 'resolved';

CREATE TABLE IF NOT EXISTS external_effect_recovery_coordinator (
  coordinator_key TEXT PRIMARY KEY CHECK(coordinator_key = 'replay'),
  lease_owner TEXT,
  lease_expires_at INTEGER NOT NULL DEFAULT 0,
  claim_epoch INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO external_effect_recovery_coordinator(
  coordinator_key, lease_owner, lease_expires_at, claim_epoch, updated_at
) VALUES ('replay', NULL, 0, 0, 0);
"#,
    )
    .map_err(|error| format!("failed to create recovery claim schema: {error}"))
}

pub(super) fn migrate_v50_to_v51(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v50->v51 transaction: {error}"))?;
    create_recovery_claim_schema(&tx)?;
    super::set_user_version(&tx, 51)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v50->v51 transaction: {error}"))?;
    Ok(())
}
