//! Usage: Rebuildable Provider daily aggregates for bounded trend queries.

use crate::shared::error::{db_err, AppResult};
use crate::{db, usage_ledger};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use std::time::Duration;

const DAY_REFRESH_PAUSE: Duration = Duration::from_millis(5);
pub(crate) const BACKGROUND_REFRESH_MAX_DAYS: usize = 32;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DailyRollupRefreshReport {
    pub(crate) rebuilt_days: u64,
    pub(crate) source_rows: u64,
    pub(crate) reset_for_calendar_change: bool,
    pub(crate) ledger_backfill_complete: bool,
    pub(crate) has_more_work: bool,
}

fn local_day(conn: &Connection, unix_seconds: i64) -> rusqlite::Result<String> {
    conn.query_row(
        "SELECT date(?1, 'unixepoch', 'localtime')",
        [unix_seconds],
        |row| row.get(0),
    )
}

fn local_day_bounds(conn: &Connection, day: &str) -> rusqlite::Result<(i64, i64)> {
    conn.query_row(
        r#"
SELECT
  CAST(strftime('%s', ?1 || ' 00:00:00', 'utc') AS INTEGER),
  CAST(strftime('%s', datetime(?1, '+1 day'), 'utc') AS INTEGER)
"#,
        [day],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

fn reset_invalid_calendar_projection(
    tx: &Transaction<'_>,
    now: i64,
) -> rusqlite::Result<bool> {
    let invalid = tx.query_row(
        r#"
SELECT EXISTS (
  SELECT 1
  FROM usage_provider_daily_rollup_days day
  WHERE day.status = 'complete'
    AND (
      day.day_start_ts != CAST(strftime(
        '%s', day.local_day || ' 00:00:00', 'utc'
      ) AS INTEGER)
      OR day.day_end_ts != CAST(strftime(
        '%s', datetime(day.local_day, '+1 day'), 'utc'
      ) AS INTEGER)
    )
)
"#,
        [],
        |row| row.get::<_, bool>(0),
    )?;
    if !invalid {
        return Ok(false);
    }

    tx.execute("DELETE FROM usage_provider_daily_rollups", [])?;
    tx.execute("DELETE FROM usage_provider_daily_rollup_days", [])?;
    tx.execute(
        r#"
UPDATE usage_provider_daily_rollup_backfill_state
SET next_local_day = NULL, updated_at = ?1
WHERE id = 1
"#,
        [now],
    )?;
    Ok(true)
}

fn initialize_backfill_cursor(
    tx: &Transaction<'_>,
    today: &str,
    now: i64,
) -> rusqlite::Result<String> {
    let existing = tx
        .query_row(
            r#"
SELECT next_local_day
FROM usage_provider_daily_rollup_backfill_state
WHERE id = 1
"#,
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    if let Some(existing) = existing {
        let valid = tx.query_row(
            "SELECT COALESCE(date(?1) = ?1, 0)",
            [&existing],
            |row| row.get::<_, bool>(0),
        )?;
        if valid && existing.as_str() <= today {
            return Ok(existing);
        }
    }

    let first_day = tx
        .query_row(
            r#"
SELECT date(MIN(created_at), 'unixepoch', 'localtime')
FROM usage_ledger
WHERE created_at > 0
"#,
            [],
            |row| row.get::<_, Option<String>>(0),
        )?
        .unwrap_or_else(|| today.to_string());
    tx.execute(
        r#"
INSERT INTO usage_provider_daily_rollup_backfill_state(
  id,
  next_local_day,
  updated_at
) VALUES (1, ?1, ?2)
ON CONFLICT(id) DO UPDATE SET
  next_local_day = excluded.next_local_day,
  updated_at = excluded.updated_at
"#,
        params![first_day, now],
    )?;
    Ok(first_day)
}

fn next_refresh_day(
    tx: &Transaction<'_>,
    cursor_day: &str,
    today: &str,
) -> rusqlite::Result<Option<String>> {
    let dirty_day = tx
        .query_row(
            r#"
SELECT local_day
FROM usage_provider_daily_rollup_days
WHERE status = 'dirty'
  AND local_day < ?1
ORDER BY local_day ASC
LIMIT 1
"#,
            [today],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let cursor_candidate = (cursor_day < today).then(|| cursor_day.to_string());
    Ok(match (dirty_day, cursor_candidate) {
        (Some(dirty), Some(cursor)) => Some(dirty.min(cursor)),
        (Some(dirty), None) => Some(dirty),
        (None, Some(cursor)) => Some(cursor),
        (None, None) => None,
    })
}

fn effective_input_tokens_sql(alias: &str) -> String {
    format!(
        "CASE WHEN ({alias}.cli_key IN ('codex', 'grok') OR {alias}.persisted_openai_input_semantics = 1) THEN MAX(COALESCE({alias}.input_tokens, 0) - COALESCE({alias}.cache_read_input_tokens, 0) - COALESCE({alias}.cache_creation_input_tokens, 0), 0) WHEN {alias}.cli_key = 'gemini' THEN MAX(COALESCE({alias}.input_tokens, 0) - COALESCE({alias}.cache_read_input_tokens, 0), 0) ELSE COALESCE({alias}.input_tokens, 0) END"
    )
}

fn rebuild_day(
    tx: &Transaction<'_>,
    day: &str,
    day_start_ts: i64,
    day_end_ts: i64,
    now: i64,
) -> AppResult<u64> {
    tx.execute(
        r#"
INSERT INTO usage_provider_daily_rollup_days(
  local_day,
  day_start_ts,
  day_end_ts,
  status,
  source_row_count,
  updated_at
) VALUES (?1, ?2, ?3, 'dirty', 0, ?4)
ON CONFLICT(local_day) DO UPDATE SET
  day_start_ts = excluded.day_start_ts,
  day_end_ts = excluded.day_end_ts,
  status = 'dirty',
  source_row_count = 0,
  updated_at = excluded.updated_at
"#,
        params![day, day_start_ts, day_end_ts, now],
    )
    .map_err(|error| db_err!("failed to prepare Provider daily rollup day: {error}"))?;
    tx.execute(
        "DELETE FROM usage_provider_daily_rollups WHERE local_day = ?1",
        [day],
    )
    .map_err(|error| db_err!("failed to clear Provider daily rollup day: {error}"))?;

    let success = "r.status >= 200 AND r.status < 300 AND r.error_present = 0";
    let valid_ttfb = "r.ttfb_ms IS NOT NULL AND r.ttfb_ms < r.duration_ms";
    let valid_output_rate =
        "r.output_tokens IS NOT NULL AND r.ttfb_ms IS NOT NULL AND r.ttfb_ms < r.duration_ms";
    let effective_input = effective_input_tokens_sql("r");
    let cache_denom = format!(
        "({effective_input}) + COALESCE(r.cache_creation_input_tokens, 0) + COALESCE(r.cache_read_input_tokens, 0)"
    );
    let sql = format!(
        r#"
INSERT INTO usage_provider_daily_rollups(
  local_day,
  cli_key,
  final_provider_id,
  provider_name_all_snapshot,
  provider_name_success_snapshot,
  created_at_min,
  created_at_max,
  requests_total,
  requests_success,
  success_duration_ms_sum,
  success_ttfb_ms_sum,
  success_ttfb_ms_count,
  success_generation_ms_sum,
  success_output_tokens_for_rate_sum,
  success_output_rate_count,
  cache_denom_tokens,
  cache_read_input_tokens
)
SELECT
  ?1,
  r.cli_key,
  r.final_provider_id,
  MAX(NULLIF(TRIM(r.provider_name_snapshot), '')),
  MAX(CASE WHEN {success} THEN NULLIF(TRIM(r.provider_name_snapshot), '') END),
  MIN(r.created_at),
  MAX(r.created_at),
  COUNT(*),
  SUM(CASE WHEN {success} THEN 1 ELSE 0 END),
  SUM(CASE WHEN {success} THEN r.duration_ms ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_ttfb} THEN r.ttfb_ms ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_ttfb} THEN 1 ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_output_rate} THEN r.duration_ms - r.ttfb_ms ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_output_rate} THEN r.output_tokens ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_output_rate} THEN 1 ELSE 0 END),
  SUM(CASE WHEN {success} THEN {cache_denom} ELSE 0 END),
  SUM(CASE WHEN {success} THEN COALESCE(r.cache_read_input_tokens, 0) ELSE 0 END)
FROM usage_ledger r
WHERE r.created_at >= ?2
  AND r.created_at < ?3
  AND r.excluded_from_stats = 0
  AND r.final_provider_id IS NOT NULL
  AND r.final_provider_id > 0
GROUP BY r.cli_key, r.final_provider_id
"#
    );
    tx.execute(&sql, params![day, day_start_ts, day_end_ts])
        .map_err(|error| db_err!("failed to aggregate Provider daily rollup: {error}"))?;

    let projected_rows = tx
        .query_row(
            r#"
SELECT COALESCE(SUM(requests_total), 0)
FROM usage_provider_daily_rollups
WHERE local_day = ?1
"#,
            [day],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| db_err!("failed to count projected daily rollup rows: {error}"))?;
    let source_rows = tx
        .query_row(
            r#"
SELECT COUNT(*)
FROM usage_ledger r
WHERE r.created_at >= ?1
  AND r.created_at < ?2
  AND r.excluded_from_stats = 0
  AND r.final_provider_id IS NOT NULL
  AND r.final_provider_id > 0
"#,
            params![day_start_ts, day_end_ts],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| db_err!("failed to count daily rollup source rows: {error}"))?;
    if projected_rows != source_rows {
        return Err(db_err!(
            "Provider daily rollup validation mismatch for {day}: source={source_rows}, projected={projected_rows}"
        ));
    }
    tx.execute(
        r#"
INSERT INTO usage_provider_daily_rollup_days(
  local_day,
  day_start_ts,
  day_end_ts,
  status,
  source_row_count,
  updated_at
) VALUES (?1, ?2, ?3, 'complete', ?4, ?5)
ON CONFLICT(local_day) DO UPDATE SET
  day_start_ts = excluded.day_start_ts,
  day_end_ts = excluded.day_end_ts,
  status = excluded.status,
  source_row_count = excluded.source_row_count,
  updated_at = excluded.updated_at
"#,
        params![day, day_start_ts, day_end_ts, source_rows, now],
    )
    .map_err(|error| db_err!("failed to complete Provider daily rollup day: {error}"))?;
    Ok(u64::try_from(source_rows.max(0)).unwrap_or(u64::MAX))
}

fn advance_cursor_after_day(
    tx: &Transaction<'_>,
    cursor_day: &str,
    rebuilt_day: &str,
    day_end_ts: i64,
    today: &str,
    now: i64,
) -> rusqlite::Result<()> {
    if cursor_day != rebuilt_day {
        return Ok(());
    }
    let next_data_day = tx
        .query_row(
            r#"
SELECT date(MIN(created_at), 'unixepoch', 'localtime')
FROM usage_ledger
WHERE created_at >= ?1
"#,
            [day_end_ts],
            |row| row.get::<_, Option<String>>(0),
        )?
        .unwrap_or_else(|| today.to_string());
    let next_day = if next_data_day.as_str() > today {
        today
    } else {
        next_data_day.as_str()
    };
    tx.execute(
        r#"
UPDATE usage_provider_daily_rollup_backfill_state
SET next_local_day = ?1, updated_at = ?2
WHERE id = 1
"#,
        params![next_day, now],
    )?;
    Ok(())
}

pub(crate) fn refresh_completed_days_batch(
    db: &db::Db,
    now: i64,
    max_days: usize,
) -> AppResult<DailyRollupRefreshReport> {
    let mut report = DailyRollupRefreshReport::default();
    {
        let conn = db.open_connection()?;
        report.ledger_backfill_complete = usage_ledger::is_backfill_complete(&conn)
            .map_err(|error| db_err!("failed to read usage ledger state for daily rollup: {error}"))?;
    }
    if !report.ledger_backfill_complete {
        return Ok(report);
    }

    while report.rebuilt_days < max_days as u64 {
        let mut conn = db.open_connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| db_err!("failed to start Provider daily rollup transaction: {error}"))?;
        if reset_invalid_calendar_projection(&tx, now).map_err(|error| {
            db_err!("failed to validate Provider daily rollup calendar: {error}")
        })? {
            report.reset_for_calendar_change = true;
        }
        let today = local_day(&tx, now)
            .map_err(|error| db_err!("failed to resolve current local day: {error}"))?;
        let cursor_day = initialize_backfill_cursor(&tx, &today, now)
            .map_err(|error| db_err!("failed to initialize daily rollup cursor: {error}"))?;
        let Some(day) = next_refresh_day(&tx, &cursor_day, &today)
            .map_err(|error| db_err!("failed to select daily rollup day: {error}"))?
        else {
            tx.commit().map_err(|error| {
                db_err!("failed to close Provider daily rollup transaction: {error}")
            })?;
            break;
        };
        let (day_start_ts, day_end_ts) = local_day_bounds(&tx, &day)
            .map_err(|error| db_err!("failed to resolve daily rollup bounds: {error}"))?;
        let source_rows = rebuild_day(&tx, &day, day_start_ts, day_end_ts, now)?;
        advance_cursor_after_day(
            &tx,
            &cursor_day,
            &day,
            day_end_ts,
            &today,
            now,
        )
        .map_err(|error| db_err!("failed to advance daily rollup cursor: {error}"))?;
        tx.commit().map_err(|error| {
            db_err!("failed to commit Provider daily rollup transaction: {error}")
        })?;
        report.rebuilt_days = report.rebuilt_days.saturating_add(1);
        report.source_rows = report.source_rows.saturating_add(source_rows);
        std::thread::sleep(DAY_REFRESH_PAUSE);
    }

    if report.rebuilt_days == max_days as u64 && max_days != usize::MAX {
        report.has_more_work = true;
    }

    Ok(report)
}

#[cfg(test)]
pub(crate) fn refresh_completed_days(
    db: &db::Db,
    now: i64,
) -> AppResult<DailyRollupRefreshReport> {
    refresh_completed_days_batch(db, now, usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn previous_local_day_fixture(
        db: &db::Db,
        now: i64,
        days_ago: i64,
    ) -> (String, i64, i64) {
        let conn = db.open_connection().expect("open fixture database");
        let day = conn
            .query_row(
                "SELECT date(?1, 'unixepoch', 'localtime', ?2)",
                params![now, format!("-{days_ago} day")],
                |row| row.get::<_, String>(0),
            )
            .expect("resolve fixture local day");
        let (start, end) = local_day_bounds(&conn, &day).expect("resolve fixture day bounds");
        (day, start, end)
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_ledger_row(
        db: &db::Db,
        request_log_id: i64,
        created_at: i64,
        status: i64,
        provider_id: i64,
        provider_name: &str,
        duration_ms: i64,
        ttfb_ms: Option<i64>,
    ) {
        let conn = db.open_connection().expect("open fixture database");
        conn.execute(
            r#"
INSERT INTO usage_ledger(
  request_log_id, trace_id, cli_key, created_at, created_at_ms, status,
  error_present, excluded_from_stats, duration_ms, ttfb_ms,
  final_provider_id, provider_name_snapshot, usage_present, input_tokens,
  output_tokens, cache_read_input_tokens, cache_creation_input_tokens
) VALUES (
  ?1, ?2, 'claude', ?3, ?4, ?5,
  0, 0, ?6, ?7,
  ?8, ?9, 1, 100,
  20, 20, 5
)
"#,
            params![
                request_log_id,
                format!("daily-rollup-{request_log_id}"),
                created_at,
                created_at.saturating_mul(1000),
                status,
                duration_ms,
                ttfb_ms,
                provider_id,
                provider_name,
            ],
        )
        .expect("insert usage ledger fixture");
    }

    #[test]
    fn refresh_rebuilds_closed_days_and_late_changes_mark_them_dirty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("daily-rollup.db"))
            .expect("init fixture database");
        let now = 1_700_000_000_i64;
        let (day, start, end) = previous_local_day_fixture(&db, now, 2);
        insert_ledger_row(&db, 1, start + 10, 200, 41, "Alpha", 100, Some(10));
        insert_ledger_row(&db, 2, start + 20, 500, 41, "Zulu", 300, Some(20));
        let conn = db.open_connection().expect("open fixture database");
        conn.execute("DELETE FROM usage_provider_daily_rollup_days", [])
            .expect("simulate pre-v46 ledger without rollup coverage");
        drop(conn);

        let report = refresh_completed_days(&db, now).expect("refresh daily rollup");
        assert_eq!(report.rebuilt_days, 1);
        assert_eq!(report.source_rows, 2);
        assert!(report.ledger_backfill_complete);

        let conn = db.open_connection().expect("open fixture database");
        let day_state: (i64, i64, String, i64) = conn
            .query_row(
                r#"
SELECT day_start_ts, day_end_ts, status, source_row_count
FROM usage_provider_daily_rollup_days
WHERE local_day = ?1
"#,
                [&day],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read rollup day state");
        assert_eq!(day_state, (start, end, "complete".to_string(), 2));
        let aggregate: (Option<String>, Option<String>, i64, i64, i64, i64, i64, i64) = conn
            .query_row(
                r#"
SELECT
  provider_name_all_snapshot,
  provider_name_success_snapshot,
  requests_total,
  requests_success,
  success_duration_ms_sum,
  success_ttfb_ms_sum,
  success_generation_ms_sum,
  cache_denom_tokens
FROM usage_provider_daily_rollups
WHERE local_day = ?1 AND final_provider_id = 41
"#,
                [&day],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .expect("read daily aggregate");
        assert_eq!(aggregate.0.as_deref(), Some("Zulu"));
        assert_eq!(aggregate.1.as_deref(), Some("Alpha"));
        assert_eq!((aggregate.2, aggregate.3), (2, 1));
        assert_eq!((aggregate.4, aggregate.5, aggregate.6), (100, 10, 90));
        assert_eq!(aggregate.7, 125);

        conn.execute(
            "UPDATE usage_ledger SET duration_ms = 150 WHERE request_log_id = 1",
            [],
        )
        .expect("apply late ledger correction");
        let status: String = conn
            .query_row(
                "SELECT status FROM usage_provider_daily_rollup_days WHERE local_day = ?1",
                [&day],
                |row| row.get(0),
            )
            .expect("read dirty day state");
        assert_eq!(status, "dirty");
        drop(conn);

        refresh_completed_days(&db, now).expect("rebuild corrected day");
        let conn = db.open_connection().expect("open fixture database");
        let duration_sum: i64 = conn
            .query_row(
                "SELECT success_duration_ms_sum FROM usage_provider_daily_rollups WHERE local_day = ?1 AND final_provider_id = 41",
                [&day],
                |row| row.get(0),
            )
            .expect("read corrected aggregate");
        assert_eq!(duration_sum, 150);
    }

    #[test]
    fn refresh_initializes_coverage_for_ledger_rows_created_before_v46() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("daily-rollup-legacy.db"))
            .expect("init fixture database");
        let now = 1_700_000_000_i64;
        let (day, start, _) = previous_local_day_fixture(&db, now, 2);
        insert_ledger_row(&db, 1, start + 10, 200, 45, "Legacy", 100, Some(10));

        let conn = db.open_connection().expect("open fixture database");
        conn.execute(
            "DELETE FROM usage_provider_daily_rollup_days WHERE local_day = ?1",
            [&day],
        )
        .expect("simulate a ledger row that predates the v46 coverage schema");
        drop(conn);

        let report = refresh_completed_days(&db, now).expect("refresh legacy ledger day");
        assert_eq!(report.rebuilt_days, 1);
        let conn = db.open_connection().expect("open fixture database");
        let state: (String, i64) = conn
            .query_row(
                r#"
SELECT d.status, COUNT(r.final_provider_id)
FROM usage_provider_daily_rollup_days d
LEFT JOIN usage_provider_daily_rollups r ON r.local_day = d.local_day
WHERE d.local_day = ?1
GROUP BY d.local_day, d.status
"#,
                [&day],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read rebuilt legacy day");
        assert_eq!(state, ("complete".to_string(), 1));
    }

    #[test]
    fn refresh_leaves_current_day_raw_and_rebuilds_invalid_calendar_bounds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("daily-rollup-calendar.db"))
            .expect("init fixture database");
        let now = 1_700_000_000_i64;
        let (closed_day, closed_start, closed_end) = previous_local_day_fixture(&db, now, 2);
        let (today, today_start, _) = previous_local_day_fixture(&db, now, 0);
        insert_ledger_row(&db, 1, closed_start + 10, 200, 51, "Closed", 100, Some(10));
        insert_ledger_row(&db, 2, today_start + 10, 200, 51, "Current", 100, Some(10));

        refresh_completed_days(&db, now).expect("refresh closed day");
        let conn = db.open_connection().expect("open fixture database");
        let current_status: String = conn
            .query_row(
                "SELECT status FROM usage_provider_daily_rollup_days WHERE local_day = ?1",
                [&today],
                |row| row.get(0),
            )
            .expect("read current day state");
        assert_eq!(current_status, "dirty");
        let current_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM usage_provider_daily_rollups WHERE local_day = ?1",
                [&today],
                |row| row.get(0),
            )
            .expect("count current rollups");
        assert_eq!(current_rows, 0);
        conn.execute(
            r#"
UPDATE usage_provider_daily_rollup_days
SET day_start_ts = day_start_ts + 1
WHERE local_day = ?1
"#,
            [&closed_day],
        )
        .expect("corrupt stored calendar boundary");
        drop(conn);

        let report = refresh_completed_days(&db, now).expect("repair calendar projection");
        assert!(report.reset_for_calendar_change);
        let conn = db.open_connection().expect("open fixture database");
        let repaired_bounds: (i64, i64) = conn
            .query_row(
                "SELECT day_start_ts, day_end_ts FROM usage_provider_daily_rollup_days WHERE local_day = ?1",
                [&closed_day],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read repaired calendar boundary");
        assert_eq!(repaired_bounds, (closed_start, closed_end));
    }

    #[test]
    fn refresh_waits_for_ledger_backfill_and_batches_days() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("daily-rollup-batch.db"))
            .expect("init fixture database");
        let now = 1_700_000_000_i64;
        let (_, first_start, _) = previous_local_day_fixture(&db, now, 3);
        let (_, second_start, _) = previous_local_day_fixture(&db, now, 2);
        insert_ledger_row(&db, 1, first_start + 10, 200, 61, "First", 100, Some(10));
        insert_ledger_row(&db, 2, second_start + 10, 200, 61, "Second", 100, Some(10));
        let conn = db.open_connection().expect("open fixture database");
        conn.execute(
            r#"
UPDATE usage_ledger_backfill_state
SET status = 'incomplete', target_request_log_id = 2,
    last_request_log_id = 0, completed_at = NULL
WHERE id = 1
"#,
            [],
        )
        .expect("mark ledger backfill incomplete");
        drop(conn);

        let waiting = refresh_completed_days_batch(&db, now, 1).expect("wait for ledger backfill");
        assert!(!waiting.ledger_backfill_complete);
        assert_eq!(waiting.rebuilt_days, 0);

        let conn = db.open_connection().expect("open fixture database");
        conn.execute(
            r#"
UPDATE usage_ledger_backfill_state
SET status = 'complete', last_request_log_id = target_request_log_id,
    completed_at = ?1, updated_at = ?1
WHERE id = 1
"#,
            [now],
        )
        .expect("complete ledger backfill");
        drop(conn);

        let first = refresh_completed_days_batch(&db, now, 1).expect("refresh first batch");
        assert_eq!(first.rebuilt_days, 1);
        assert!(first.has_more_work);
        let second = refresh_completed_days_batch(&db, now, 1).expect("refresh second batch");
        assert_eq!(second.rebuilt_days, 1);
    }

    #[test]
    fn deleting_ledger_rows_rebuilds_the_day_to_an_empty_projection() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("daily-rollup-delete.db"))
            .expect("init fixture database");
        let now = 1_700_000_000_i64;
        let (day, start, _) = previous_local_day_fixture(&db, now, 2);
        insert_ledger_row(&db, 1, start + 10, 200, 71, "Deleted", 100, Some(10));
        refresh_completed_days(&db, now).expect("create daily aggregate");

        let conn = db.open_connection().expect("open fixture database");
        conn.execute("DELETE FROM usage_ledger WHERE request_log_id = 1", [])
            .expect("delete ledger row");
        let dirty_status: String = conn
            .query_row(
                "SELECT status FROM usage_provider_daily_rollup_days WHERE local_day = ?1",
                [&day],
                |row| row.get(0),
            )
            .expect("read dirty day after delete");
        assert_eq!(dirty_status, "dirty");
        drop(conn);

        refresh_completed_days(&db, now).expect("rebuild empty daily aggregate");
        let conn = db.open_connection().expect("open fixture database");
        let state: (String, i64, i64) = conn
            .query_row(
                r#"
SELECT
  status,
  source_row_count,
  (SELECT COUNT(*) FROM usage_provider_daily_rollups WHERE local_day = ?1)
FROM usage_provider_daily_rollup_days
WHERE local_day = ?1
"#,
                [&day],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read empty rebuilt day");
        assert_eq!(state, ("complete".to_string(), 0, 0));
    }
}
