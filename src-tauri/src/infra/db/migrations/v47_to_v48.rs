//! Usage: SQLite migration v47->v48 - final-upstream stream timing semantics.

use rusqlite::Connection;

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let sql = format!("SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1)");
    conn.query_row(&sql, [column], |row| row.get(0))
        .map_err(|error| format!("failed to inspect {table}.{column}: {error}"))
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    if has_column(conn, table, column)? {
        return Ok(());
    }
    conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {definition};"))
        .map_err(|error| format!("failed to add {table}.{column}: {error}"))
}

pub(super) fn ensure_request_log_stream_timing_columns(
    conn: &Connection,
) -> Result<(), String> {
    add_column_if_missing(
        conn,
        "request_logs",
        "upstream_stream_duration_ms",
        "upstream_stream_duration_ms INTEGER",
    )?;
    add_column_if_missing(
        conn,
        "request_logs",
        "upstream_stream_timing_version",
        "upstream_stream_timing_version INTEGER NOT NULL DEFAULT 0 CHECK(upstream_stream_timing_version IN (0, 1))",
    )
}

pub(super) fn ensure_usage_ledger_stream_timing_columns(
    conn: &Connection,
) -> Result<(), String> {
    add_column_if_missing(
        conn,
        "usage_ledger",
        "upstream_stream_duration_ms",
        "upstream_stream_duration_ms INTEGER",
    )?;
    add_column_if_missing(
        conn,
        "usage_ledger",
        "upstream_stream_timing_version",
        "upstream_stream_timing_version INTEGER NOT NULL DEFAULT 0 CHECK(upstream_stream_timing_version IN (0, 1))",
    )
}

pub(super) fn migrate_v47_to_v48(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v47->v48 transaction: {error}"))?;

    ensure_request_log_stream_timing_columns(&tx)?;
    ensure_usage_ledger_stream_timing_columns(&tx)?;

    super::v42_to_v43::recreate_usage_events_view(&tx)?;
    super::v46_to_v47::create_provider_daily_rollup_schema(&tx)?;

    tx.execute("DELETE FROM usage_provider_daily_rollups", [])
        .map_err(|error| format!("failed to clear stale provider daily rollups: {error}"))?;
    tx.execute(
        "UPDATE usage_provider_daily_rollup_days SET status = 'dirty', source_row_count = 0, updated_at = CAST(strftime('%s', 'now') AS INTEGER)",
        [],
    )
    .map_err(|error| format!("failed to mark provider daily rollups dirty: {error}"))?;
    tx.execute(
        "UPDATE usage_provider_daily_rollup_backfill_state SET next_local_day = NULL, updated_at = CAST(strftime('%s', 'now') AS INTEGER) WHERE id = 1",
        [],
    )
    .map_err(|error| format!("failed to reset provider daily rollup backfill state: {error}"))?;

    super::set_user_version(&tx, 48)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v47->v48 transaction: {error}"))?;
    Ok(())
}
