//! Usage: SQLite migration v46->v47 - rebuildable Provider daily trend rollups.

use rusqlite::Connection;

fn add_ledger_column_if_missing(
    conn: &Connection,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let exists = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('usage_ledger') WHERE name = ?1)",
            [column],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| format!("failed to inspect usage_ledger.{column}: {error}"))?;
    if !exists {
        conn.execute_batch(&format!(
            "ALTER TABLE usage_ledger ADD COLUMN {definition};"
        ))
        .map_err(|error| format!("failed to add usage_ledger.{column}: {error}"))?;
    }
    Ok(())
}

pub(super) fn create_provider_daily_rollup_schema(conn: &Connection) -> Result<(), String> {
    super::v48_to_v49::ensure_usage_ledger_final_attempt_timing_columns(conn)?;
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS usage_provider_daily_rollup_days (
  local_day TEXT PRIMARY KEY,
  day_start_ts INTEGER NOT NULL,
  day_end_ts INTEGER NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('dirty', 'complete')),
  source_row_count INTEGER NOT NULL DEFAULT 0 CHECK(source_row_count >= 0),
  updated_at INTEGER NOT NULL,
  CHECK(length(local_day) = 10),
  CHECK(day_end_ts > day_start_ts)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS usage_provider_daily_rollups (
  local_day TEXT NOT NULL,
  cli_key TEXT NOT NULL,
  final_provider_id INTEGER NOT NULL CHECK(final_provider_id > 0),
  provider_name_all_snapshot TEXT,
  provider_name_success_snapshot TEXT,
  created_at_min INTEGER NOT NULL,
  created_at_max INTEGER NOT NULL,
  requests_total INTEGER NOT NULL,
  requests_success INTEGER NOT NULL,
  success_duration_ms_sum INTEGER NOT NULL,
  success_ttfb_ms_sum INTEGER NOT NULL,
  success_ttfb_ms_count INTEGER NOT NULL,
  success_generation_ms_sum INTEGER NOT NULL,
  success_output_tokens_for_rate_sum INTEGER NOT NULL,
  success_output_rate_count INTEGER NOT NULL,
  success_output_tokens_per_second_sum REAL NOT NULL DEFAULT 0
    CHECK(success_output_tokens_per_second_sum >= 0),
  cache_denom_tokens INTEGER NOT NULL,
  cache_read_input_tokens INTEGER NOT NULL,
  PRIMARY KEY(local_day, cli_key, final_provider_id),
  FOREIGN KEY(local_day) REFERENCES usage_provider_daily_rollup_days(local_day)
    ON DELETE CASCADE
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS idx_usage_provider_daily_rollups_provider_day
  ON usage_provider_daily_rollups(final_provider_id, local_day);
CREATE INDEX IF NOT EXISTS idx_usage_provider_daily_rollups_cli_day
  ON usage_provider_daily_rollups(cli_key, local_day);
CREATE INDEX IF NOT EXISTS idx_usage_provider_daily_rollup_days_status_day
  ON usage_provider_daily_rollup_days(status, local_day);

CREATE TABLE IF NOT EXISTS usage_provider_daily_rollup_backfill_state (
  id INTEGER PRIMARY KEY CHECK(id = 1),
  next_local_day TEXT,
  updated_at INTEGER NOT NULL
) WITHOUT ROWID;

INSERT OR IGNORE INTO usage_provider_daily_rollup_backfill_state(
  id,
  next_local_day,
  updated_at
) VALUES (
  1,
  NULL,
  CAST(strftime('%s', 'now') AS INTEGER)
);

-- Recreate the triggers from the canonical definition on every ensure pass.
-- The date guards make projection maintenance fail-open for malformed legacy
-- timestamps: those rows remain available through the raw query path without
-- blocking the primary usage-ledger write.
DROP TRIGGER IF EXISTS trg_usage_ledger_daily_rollup_insert;
DROP TRIGGER IF EXISTS trg_usage_ledger_daily_rollup_update;
DROP TRIGGER IF EXISTS trg_usage_ledger_daily_rollup_delete;

CREATE TRIGGER trg_usage_ledger_daily_rollup_insert
AFTER INSERT ON usage_ledger
WHEN NEW.created_at > 0
  AND date(NEW.created_at, 'unixepoch', 'localtime') IS NOT NULL
  AND NOT EXISTS (
    SELECT 1
    FROM usage_provider_daily_rollup_days day
    WHERE day.local_day = date(NEW.created_at, 'unixepoch', 'localtime')
      AND day.status = 'dirty'
  )
  AND strftime(
    '%s', date(NEW.created_at, 'unixepoch', 'localtime'), 'utc'
  ) IS NOT NULL
  AND strftime(
    '%s', date(NEW.created_at, 'unixepoch', 'localtime', '+1 day'), 'utc'
  ) IS NOT NULL
  AND CAST(strftime(
    '%s', date(NEW.created_at, 'unixepoch', 'localtime', '+1 day'), 'utc'
  ) AS INTEGER) > CAST(strftime(
    '%s', date(NEW.created_at, 'unixepoch', 'localtime'), 'utc'
  ) AS INTEGER)
BEGIN
  INSERT INTO usage_provider_daily_rollup_days(
    local_day,
    day_start_ts,
    day_end_ts,
    status,
    source_row_count,
    updated_at
  ) VALUES (
    date(NEW.created_at, 'unixepoch', 'localtime'),
    CAST(strftime(
      '%s',
      date(NEW.created_at, 'unixepoch', 'localtime'),
      'utc'
    ) AS INTEGER),
    CAST(strftime(
      '%s',
      date(NEW.created_at, 'unixepoch', 'localtime', '+1 day'),
      'utc'
    ) AS INTEGER),
    'dirty',
    0,
    CAST(strftime('%s', 'now') AS INTEGER)
  )
  ON CONFLICT(local_day) DO UPDATE SET
    day_start_ts = excluded.day_start_ts,
    day_end_ts = excluded.day_end_ts,
    status = 'dirty',
    updated_at = excluded.updated_at
  WHERE usage_provider_daily_rollup_days.status = 'complete'
     OR usage_provider_daily_rollup_days.day_start_ts != excluded.day_start_ts
     OR usage_provider_daily_rollup_days.day_end_ts != excluded.day_end_ts;
END;

CREATE TRIGGER trg_usage_ledger_daily_rollup_update
AFTER UPDATE OF
  created_at,
  cli_key,
  final_provider_id,
  provider_name_snapshot,
  status,
  error_present,
  excluded_from_stats,
  duration_ms,
  ttfb_ms,
  visible_ttfb_ms,
  upstream_stream_duration_ms,
  upstream_stream_timing_version,
  final_upstream_attempt_duration_ms,
  final_upstream_attempt_timing_version,
  input_tokens,
  output_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  persisted_openai_input_semantics
ON usage_ledger
WHEN OLD.created_at IS NOT NEW.created_at
  OR OLD.cli_key IS NOT NEW.cli_key
  OR OLD.final_provider_id IS NOT NEW.final_provider_id
  OR OLD.provider_name_snapshot IS NOT NEW.provider_name_snapshot
  OR OLD.status IS NOT NEW.status
  OR OLD.error_present IS NOT NEW.error_present
  OR OLD.excluded_from_stats IS NOT NEW.excluded_from_stats
  OR OLD.duration_ms IS NOT NEW.duration_ms
  OR OLD.ttfb_ms IS NOT NEW.ttfb_ms
  OR OLD.visible_ttfb_ms IS NOT NEW.visible_ttfb_ms
  OR OLD.upstream_stream_duration_ms IS NOT NEW.upstream_stream_duration_ms
  OR OLD.upstream_stream_timing_version IS NOT NEW.upstream_stream_timing_version
  OR OLD.final_upstream_attempt_duration_ms IS NOT NEW.final_upstream_attempt_duration_ms
  OR OLD.final_upstream_attempt_timing_version IS NOT NEW.final_upstream_attempt_timing_version
  OR OLD.input_tokens IS NOT NEW.input_tokens
  OR OLD.output_tokens IS NOT NEW.output_tokens
  OR OLD.cache_read_input_tokens IS NOT NEW.cache_read_input_tokens
  OR OLD.cache_creation_input_tokens IS NOT NEW.cache_creation_input_tokens
  OR OLD.persisted_openai_input_semantics IS NOT NEW.persisted_openai_input_semantics
BEGIN
  INSERT INTO usage_provider_daily_rollup_days(
    local_day,
    day_start_ts,
    day_end_ts,
    status,
    source_row_count,
    updated_at
  )
  SELECT
    date(OLD.created_at, 'unixepoch', 'localtime'),
    CAST(strftime(
      '%s',
      date(OLD.created_at, 'unixepoch', 'localtime'),
      'utc'
    ) AS INTEGER),
    CAST(strftime(
      '%s',
      date(OLD.created_at, 'unixepoch', 'localtime', '+1 day'),
      'utc'
    ) AS INTEGER),
    'dirty',
    0,
    CAST(strftime('%s', 'now') AS INTEGER)
  WHERE OLD.created_at > 0
    AND date(OLD.created_at, 'unixepoch', 'localtime') IS NOT NULL
    AND NOT EXISTS (
      SELECT 1
      FROM usage_provider_daily_rollup_days day
      WHERE day.local_day = date(OLD.created_at, 'unixepoch', 'localtime')
        AND day.status = 'dirty'
    )
    AND strftime(
      '%s', date(OLD.created_at, 'unixepoch', 'localtime'), 'utc'
    ) IS NOT NULL
    AND strftime(
      '%s', date(OLD.created_at, 'unixepoch', 'localtime', '+1 day'), 'utc'
    ) IS NOT NULL
    AND CAST(strftime(
      '%s', date(OLD.created_at, 'unixepoch', 'localtime', '+1 day'), 'utc'
    ) AS INTEGER) > CAST(strftime(
      '%s', date(OLD.created_at, 'unixepoch', 'localtime'), 'utc'
    ) AS INTEGER)
  ON CONFLICT(local_day) DO UPDATE SET
    day_start_ts = excluded.day_start_ts,
    day_end_ts = excluded.day_end_ts,
    status = 'dirty',
    updated_at = excluded.updated_at
  WHERE usage_provider_daily_rollup_days.status = 'complete'
     OR usage_provider_daily_rollup_days.day_start_ts != excluded.day_start_ts
     OR usage_provider_daily_rollup_days.day_end_ts != excluded.day_end_ts;

  INSERT INTO usage_provider_daily_rollup_days(
    local_day,
    day_start_ts,
    day_end_ts,
    status,
    source_row_count,
    updated_at
  )
  SELECT
    date(NEW.created_at, 'unixepoch', 'localtime'),
    CAST(strftime(
      '%s',
      date(NEW.created_at, 'unixepoch', 'localtime'),
      'utc'
    ) AS INTEGER),
    CAST(strftime(
      '%s',
      date(NEW.created_at, 'unixepoch', 'localtime', '+1 day'),
      'utc'
    ) AS INTEGER),
    'dirty',
    0,
    CAST(strftime('%s', 'now') AS INTEGER)
  WHERE NEW.created_at > 0
    AND date(NEW.created_at, 'unixepoch', 'localtime') IS NOT NULL
    AND NOT EXISTS (
      SELECT 1
      FROM usage_provider_daily_rollup_days day
      WHERE day.local_day = date(NEW.created_at, 'unixepoch', 'localtime')
        AND day.status = 'dirty'
    )
    AND strftime(
      '%s', date(NEW.created_at, 'unixepoch', 'localtime'), 'utc'
    ) IS NOT NULL
    AND strftime(
      '%s', date(NEW.created_at, 'unixepoch', 'localtime', '+1 day'), 'utc'
    ) IS NOT NULL
    AND CAST(strftime(
      '%s', date(NEW.created_at, 'unixepoch', 'localtime', '+1 day'), 'utc'
    ) AS INTEGER) > CAST(strftime(
      '%s', date(NEW.created_at, 'unixepoch', 'localtime'), 'utc'
    ) AS INTEGER)
  ON CONFLICT(local_day) DO UPDATE SET
    day_start_ts = excluded.day_start_ts,
    day_end_ts = excluded.day_end_ts,
    status = 'dirty',
    updated_at = excluded.updated_at
  WHERE usage_provider_daily_rollup_days.status = 'complete'
     OR usage_provider_daily_rollup_days.day_start_ts != excluded.day_start_ts
     OR usage_provider_daily_rollup_days.day_end_ts != excluded.day_end_ts;
END;

CREATE TRIGGER trg_usage_ledger_daily_rollup_delete
AFTER DELETE ON usage_ledger
WHEN OLD.created_at > 0
  AND date(OLD.created_at, 'unixepoch', 'localtime') IS NOT NULL
  AND NOT EXISTS (
    SELECT 1
    FROM usage_provider_daily_rollup_days day
    WHERE day.local_day = date(OLD.created_at, 'unixepoch', 'localtime')
      AND day.status = 'dirty'
  )
  AND strftime(
    '%s', date(OLD.created_at, 'unixepoch', 'localtime'), 'utc'
  ) IS NOT NULL
  AND strftime(
    '%s', date(OLD.created_at, 'unixepoch', 'localtime', '+1 day'), 'utc'
  ) IS NOT NULL
  AND CAST(strftime(
    '%s', date(OLD.created_at, 'unixepoch', 'localtime', '+1 day'), 'utc'
  ) AS INTEGER) > CAST(strftime(
    '%s', date(OLD.created_at, 'unixepoch', 'localtime'), 'utc'
  ) AS INTEGER)
BEGIN
  INSERT INTO usage_provider_daily_rollup_days(
    local_day,
    day_start_ts,
    day_end_ts,
    status,
    source_row_count,
    updated_at
  ) VALUES (
    date(OLD.created_at, 'unixepoch', 'localtime'),
    CAST(strftime(
      '%s',
      date(OLD.created_at, 'unixepoch', 'localtime'),
      'utc'
    ) AS INTEGER),
    CAST(strftime(
      '%s',
      date(OLD.created_at, 'unixepoch', 'localtime', '+1 day'),
      'utc'
    ) AS INTEGER),
    'dirty',
    0,
    CAST(strftime('%s', 'now') AS INTEGER)
  )
  ON CONFLICT(local_day) DO UPDATE SET
    day_start_ts = excluded.day_start_ts,
    day_end_ts = excluded.day_end_ts,
    status = 'dirty',
    updated_at = excluded.updated_at
  WHERE usage_provider_daily_rollup_days.status = 'complete'
     OR usage_provider_daily_rollup_days.day_start_ts != excluded.day_start_ts
     OR usage_provider_daily_rollup_days.day_end_ts != excluded.day_end_ts;
END;
"#,
    )
    .map_err(|error| format!("failed to create Provider daily rollup schema: {error}"))
}

pub(super) fn migrate_v46_to_v47(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v46->v47 transaction: {error}"))?;
    let ledger_existed = tx
        .query_row(
            r#"
SELECT EXISTS(
  SELECT 1
  FROM sqlite_master
  WHERE type = 'table' AND name = 'usage_ledger'
)
"#,
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| format!("failed to inspect usage ledger before v46->v47: {error}"))?;
    // Keep upgrades recoverable when a v45 database has lost its derived
    // ledger table. Drop any stale complete marker so the post-migration
    // ensure pass captures a new fixed high-water mark before retention can
    // remove the only surviving request-log facts.
    super::v42_to_v43::create_usage_ledger_schema(&tx)?;
    // The canonical trigger is also reused by v48 and therefore references
    // final-upstream timing columns. Existing v46 ledgers need those columns
    // before the v47 trigger can be parsed; v48 will fill the request-log side.
    add_ledger_column_if_missing(
        &tx,
        "upstream_stream_duration_ms",
        "upstream_stream_duration_ms INTEGER",
    )?;
    add_ledger_column_if_missing(
        &tx,
        "upstream_stream_timing_version",
        "upstream_stream_timing_version INTEGER NOT NULL DEFAULT 0 CHECK(upstream_stream_timing_version IN (0, 1))",
    )?;
    if !ledger_existed {
        tx.execute("DELETE FROM usage_ledger_backfill_state", [])
            .map_err(|error| {
                format!("failed to reset stale usage ledger backfill state: {error}")
            })?;
    }
    create_provider_daily_rollup_schema(&tx)?;
    super::set_user_version(&tx, 47)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v46->v47 transaction: {error}"))?;
    Ok(())
}
