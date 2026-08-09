//! Usage: SQLite migration v51->v52 - scheduled provider probes and arithmetic TPS rollups.

use rusqlite::{Connection, OptionalExtension};

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        [table],
        |_row| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(|error| format!("failed to inspect table {table}: {error}"))
}

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
) -> Result<bool, String> {
    if column_exists(conn, table, column)? {
        return Ok(false);
    }
    conn.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition};"
    ))
    .map_err(|error| format!("failed to add {table}.{column}: {error}"))?;
    Ok(true)
}

pub(super) fn ensure_provider_availability_probe_columns(
    conn: &Connection,
) -> Result<(), String> {
    if !table_exists(conn, "providers")? {
        return Ok(());
    }
    add_column_if_missing(
        conn,
        "providers",
        "availability_probe_enabled",
        "INTEGER NOT NULL DEFAULT 0 CHECK(availability_probe_enabled IN (0, 1))",
    )?;
    add_column_if_missing(
        conn,
        "providers",
        "availability_probe_interval_minutes",
        "INTEGER NOT NULL DEFAULT 10 CHECK(availability_probe_interval_minutes BETWEEN 1 AND 1440)",
    )?;
    Ok(())
}

pub(super) fn ensure_usage_provider_daily_rollup_tps_sum_column(
    conn: &Connection,
) -> Result<bool, String> {
    if !table_exists(conn, "usage_provider_daily_rollups")? {
        return Ok(false);
    }
    add_column_if_missing(
        conn,
        "usage_provider_daily_rollups",
        "success_output_tokens_per_second_sum",
        "REAL NOT NULL DEFAULT 0 CHECK(success_output_tokens_per_second_sum >= 0)",
    )
}

pub(super) fn reset_provider_daily_rollup_projection(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "usage_provider_daily_rollups")? {
        return Ok(());
    }
    conn.execute("DELETE FROM usage_provider_daily_rollups", [])
        .map_err(|error| format!("failed to clear stale Provider daily rollups: {error}"))?;
    conn.execute(
        "UPDATE usage_provider_daily_rollup_days SET status = 'dirty', source_row_count = 0, updated_at = CAST(strftime('%s', 'now') AS INTEGER)",
        [],
    )
    .map_err(|error| format!("failed to mark Provider daily rollups dirty: {error}"))?;
    conn.execute(
        "UPDATE usage_provider_daily_rollup_backfill_state SET next_local_day = NULL, updated_at = CAST(strftime('%s', 'now') AS INTEGER) WHERE id = 1",
        [],
    )
    .map_err(|error| format!("failed to reset Provider daily rollup cursor: {error}"))?;
    Ok(())
}

pub(super) fn migrate_v51_to_v52(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v51->v52 transaction: {error}"))?;
    ensure_provider_availability_probe_columns(&tx)?;
    super::v46_to_v47::create_provider_daily_rollup_schema(&tx)?;
    ensure_usage_provider_daily_rollup_tps_sum_column(&tx)?;
    reset_provider_daily_rollup_projection(&tx)?;
    super::set_user_version(&tx, 52)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v51->v52 transaction: {error}"))?;
    Ok(())
}
