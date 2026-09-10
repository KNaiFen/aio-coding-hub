//! Usage: SQLite migration v54->v55 - independent provider spend-limit resets.

use rusqlite::Connection;

pub(super) fn ensure_provider_limit_resets(conn: &Connection) -> crate::shared::error::AppResult<()> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS provider_limit_resets (
  provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
  period TEXT NOT NULL CHECK(period IN ('5h', 'daily', 'weekly', 'monthly')),
  reset_at INTEGER,
  request_log_id_cutoff INTEGER NOT NULL CHECK(request_log_id_cutoff >= 0),
  PRIMARY KEY(provider_id, period)
);
"#,
    )
    .map_err(|error| format!("failed to create provider limit resets: {error}"))?;
    Ok(())
}

pub(super) fn migrate_v54_to_v55(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v54->v55 transaction: {error}"))?;
    ensure_provider_limit_resets(&tx)?;
    super::set_user_version(&tx, 55)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v54->v55 migration: {error}"))?;
    Ok(())
}
