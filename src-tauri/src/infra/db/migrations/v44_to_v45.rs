//! Usage: SQLite migration v44->v45 - short-lived provider availability facts.

use rusqlite::Connection;

pub(super) fn migrate_v44_to_v45(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start sqlite transaction: {error}"))?;
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS provider_availability_observations (
  trace_id TEXT NOT NULL,
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  observed_at_ms INTEGER NOT NULL,
  success INTEGER NOT NULL CHECK(success IN (0, 1)),
  PRIMARY KEY(trace_id, provider_id),
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_provider_availability_provider_time
ON provider_availability_observations(provider_id, observed_at_ms);

CREATE INDEX IF NOT EXISTS idx_provider_availability_observed_at
ON provider_availability_observations(observed_at_ms);
"#,
    )
    .map_err(|error| format!("failed to create provider availability observations: {error}"))?;
    super::set_user_version(&tx, 45)?;
    tx.commit()
        .map_err(|error| format!("failed to commit sqlite transaction: {error}"))?;
    Ok(())
}
