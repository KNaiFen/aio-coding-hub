//! Usage: SQLite migration v41->v42 - Per-route session reuse priorities.

use rusqlite::Connection;

const PRIORITY_COLUMN: &str = "session_reuse_priority";
const MAX_SESSION_REUSE_PRIORITY: i64 = 1000;

pub(super) fn migrate_v41_to_v42(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v41->v42: {error}"))?;

    for table in ["default_route_providers", "sort_mode_providers"] {
        // Dev and legacy schemas can omit route tables that the idempotent ensure
        // phase creates after versioned migrations. Upgrade present tables here
        // and let that phase repair the missing ones.
        if table_exists(&tx, table)? && !has_column(&tx, table, PRIORITY_COLUMN)? {
            tx.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {PRIORITY_COLUMN} INTEGER NOT NULL DEFAULT 0 \
                 CHECK({PRIORITY_COLUMN} BETWEEN 0 AND {MAX_SESSION_REUSE_PRIORITY});"
            ))
            .map_err(|error| {
                format!("failed to add {table}.{PRIORITY_COLUMN} during v41->v42: {error}")
            })?;
        }
    }

    super::set_user_version(&tx, 42)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v41->v42: {error}"))?;
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )
    .map(|exists: i64| exists != 0)
    .map_err(|error| format!("failed to inspect {table} during v41->v42: {error}"))
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
        [table, column],
        |row| row.get(0),
    )
    .map_err(|error| format!("failed to inspect {table}.{column} during v41->v42: {error}"))
}
