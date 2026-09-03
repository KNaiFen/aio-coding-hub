//! Usage: SQLite migration v53->v54 - same-boundary final attempt estimate.

use rusqlite::Connection;

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
            (table, column),
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to inspect {table}.{column}: {error}"))?;
    if !exists {
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition};"))
        .map_err(|error| format!("failed to add {table}.{column}: {error}"))?;
    }
    Ok(())
}

pub(super) fn migrate_v53_to_v54(
    conn: &mut Connection,
) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v53->v54 transaction: {error}"))?;
    add_column_if_missing(
        &tx,
        "request_logs",
        "estimated_final_upstream_attempt_duration_ms",
        "INTEGER",
    )?;
    super::set_user_version(&tx, 54)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v53->v54 migration: {error}"))?;
    Ok(())
}
