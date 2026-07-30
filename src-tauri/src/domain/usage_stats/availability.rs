use crate::db;
use crate::shared::error::db_err;
use crate::shared::time::now_unix_millis;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

const TARGET_BUCKETS: i64 = 60;
const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;
const DAY_MS: i64 = 24 * HOUR_MS;

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageAvailabilityParams {
    pub lookback_ms: Option<i64>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub cli_key: Option<String>,
    pub provider_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct UsageAvailabilityBucketV1 {
    pub cli_key: String,
    pub provider_id: i64,
    pub provider_name: String,
    pub bucket_start_ms: i64,
    pub requests_total: i64,
    pub requests_success: i64,
    pub total_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct UsageAvailabilityTimelineV1 {
    pub start_ms: i64,
    pub end_ms: i64,
    pub bucket_size_ms: i64,
    pub buckets: Vec<UsageAvailabilityBucketV1>,
}

fn invalid_input(message: &str) -> crate::shared::error::AppError {
    crate::shared::error::AppError::new("SEC_INVALID_INPUT", message)
}

fn resolve_bounds(
    params: &UsageAvailabilityParams,
    now_ms: i64,
) -> crate::shared::error::AppResult<(i64, i64)> {
    match (params.lookback_ms, params.start_ms, params.end_ms) {
        (Some(lookback_ms), None, None) if lookback_ms > 0 => {
            Ok((now_ms.saturating_sub(lookback_ms), now_ms))
        }
        (None, Some(start_ms), Some(end_ms))
            if start_ms >= 0 && end_ms >= 0 && start_ms < end_ms =>
        {
            Ok((start_ms, end_ms))
        }
        _ => Err(invalid_input(
            "availability requires either positive lookback_ms or valid start_ms/end_ms",
        )),
    }
}

fn bucket_size_ms(range_ms: i64) -> i64 {
    let raw = range_ms as f64 / TARGET_BUCKETS as f64;
    [
        5 * MINUTE_MS,
        10 * MINUTE_MS,
        15 * MINUTE_MS,
        24 * MINUTE_MS,
        30 * MINUTE_MS,
        HOUR_MS,
        2 * HOUR_MS,
        4 * HOUR_MS,
        6 * HOUR_MS,
        12 * HOUR_MS,
        DAY_MS,
    ]
    .into_iter()
    .find(|candidate| *candidate as f64 >= raw)
    .unwrap_or(DAY_MS)
}

fn availability_timeline_v1_with_conn(
    conn: &Connection,
    params: &UsageAvailabilityParams,
    now_ms: i64,
) -> crate::shared::error::AppResult<UsageAvailabilityTimelineV1> {
    let (start_ms, end_ms) = resolve_bounds(params, now_ms)?;
    let cli_key = match params.cli_key.as_deref() {
        Some(cli_key) => {
            crate::shared::cli_key::validate_cli_key(cli_key)?;
            Some(cli_key)
        }
        None => None,
    };
    if params
        .provider_id
        .is_some_and(|provider_id| provider_id <= 0)
    {
        return Err(invalid_input("availability provider_id must be > 0"));
    }

    let size_ms = bucket_size_ms(end_ms.saturating_sub(start_ms));
    let bucket_count = end_ms.saturating_sub(start_ms).saturating_add(size_ms - 1) / size_ms;

    // usage_ledger intentionally does not persist request path. Availability
    // therefore uses the analytics event set instead of the legacy
    // CLAUDE_VISIBLE_LOG_CONDITION path filter, excluding probes/system rows
    // through excluded_from_stats.
    let mut stmt = conn
        .prepare(
            r#"
WITH filtered AS (
  SELECT
    id,
    cli_key,
    COALESCE(final_provider_id, 0) AS provider_id,
    COALESCE(NULLIF(TRIM(provider_name_snapshot), ''), 'Unknown') AS provider_name,
    status,
    duration_ms,
    CASE
      WHEN created_at_ms > 0 THEN created_at_ms
      ELSE created_at * 1000
    END AS event_ms
  FROM usage_events
  WHERE excluded_from_stats = 0
    AND CASE
      WHEN created_at_ms > 0 THEN created_at_ms
      ELSE created_at * 1000
    END >= ?1
    AND CASE
      WHEN created_at_ms > 0 THEN created_at_ms
      ELSE created_at * 1000
    END <= ?2
    AND created_at >= ?5
    AND created_at <= ?6
    AND (?7 IS NULL OR cli_key = ?7)
    AND (?8 IS NULL OR COALESCE(final_provider_id, 0) = ?8)
),
named AS (
  SELECT
    *,
    FIRST_VALUE(provider_name) OVER (
      PARTITION BY cli_key, provider_id
      ORDER BY event_ms DESC, id DESC
    ) AS latest_provider_name
  FROM filtered
),
bucketed AS (
  SELECT
    cli_key,
    provider_id,
    latest_provider_name AS provider_name,
    ?1 + MIN(
      CAST((event_ms - ?1) / ?3 AS INTEGER),
      ?4 - 1
    ) * ?3 AS bucket_start_ms,
    status,
    duration_ms
  FROM named
)
SELECT
  cli_key,
  provider_id,
  provider_name,
  bucket_start_ms,
  COUNT(*) AS requests_total,
  SUM(CASE WHEN status >= 200 AND status < 400 THEN 1 ELSE 0 END) AS requests_success,
  TOTAL(duration_ms) AS total_duration_ms
FROM bucketed
GROUP BY cli_key, provider_id, provider_name, bucket_start_ms
ORDER BY bucket_start_ms ASC, cli_key ASC, provider_id ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare availability timeline query: {e}"))?;

    let rows = stmt
        .query_map(
            params![
                start_ms,
                end_ms,
                size_ms,
                bucket_count.max(1),
                start_ms.div_euclid(1000),
                end_ms.saturating_add(999).div_euclid(1000),
                cli_key,
                params.provider_id
            ],
            |row| {
                Ok(UsageAvailabilityBucketV1 {
                    cli_key: row.get("cli_key")?,
                    provider_id: row.get("provider_id")?,
                    provider_name: row.get("provider_name")?,
                    bucket_start_ms: row.get("bucket_start_ms")?,
                    requests_total: row.get("requests_total")?,
                    requests_success: row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0),
                    total_duration_ms: row.get("total_duration_ms")?,
                })
            },
        )
        .map_err(|e| db_err!("failed to query availability timeline: {e}"))?;

    let mut buckets = Vec::new();
    for row in rows {
        buckets.push(row.map_err(|e| db_err!("failed to read availability timeline bucket: {e}"))?);
    }

    Ok(UsageAvailabilityTimelineV1 {
        start_ms,
        end_ms,
        bucket_size_ms: size_ms,
        buckets,
    })
}

pub fn availability_timeline_v1(
    db: &db::Db,
    params: &UsageAvailabilityParams,
) -> crate::shared::error::AppResult<UsageAvailabilityTimelineV1> {
    let conn = db.open_connection()?;
    availability_timeline_v1_with_conn(&conn, params, now_unix_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn insert_ledger_event(
        conn: &Connection,
        request_log_id: i64,
        provider_name: &str,
        status: Option<i64>,
        error_present: i64,
        duration_ms: i64,
        created_at_ms: i64,
        excluded_from_stats: i64,
    ) {
        conn.execute(
            r#"
INSERT INTO usage_ledger (
  request_log_id, trace_id, cli_key, created_at, created_at_ms, status,
  error_present, excluded_from_stats, duration_ms, final_provider_id,
  provider_name_snapshot
) VALUES (?1, ?2, 'codex', ?3, ?4, ?5, ?6, ?7, ?8, 42, ?9)
"#,
            params![
                request_log_id,
                format!("availability-{request_log_id}"),
                created_at_ms / 1000,
                created_at_ms,
                status,
                error_present,
                excluded_from_stats,
                duration_ms,
                provider_name
            ],
        )
        .expect("insert usage ledger event");
    }

    #[test]
    fn availability_reads_compacted_ledger_and_preserves_timeline_semantics() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db =
            crate::db::init_for_tests(&dir.path().join("availability-ledger.db")).expect("init db");
        let conn = db.open_connection().expect("open db");
        conn.execute(
            "UPDATE usage_ledger_backfill_state SET status = 'complete' WHERE id = 1",
            [],
        )
        .expect("complete backfill");

        insert_ledger_event(&conn, 1, "Old Name", Some(500), 0, 300, 110_000, 0);
        insert_ledger_event(&conn, 2, "New Name", Some(302), 1, 100, 120_000, 0);
        insert_ledger_event(&conn, 3, "Excluded Probe", Some(200), 0, 9_999, 125_000, 1);
        conn.execute("DELETE FROM request_logs", [])
            .expect("remove compacted request details");

        let result = availability_timeline_v1_with_conn(
            &conn,
            &UsageAvailabilityParams {
                lookback_ms: None,
                start_ms: Some(100_000),
                end_ms: Some(200_000),
                cli_key: Some("codex".to_string()),
                provider_id: Some(42),
            },
            999_999,
        )
        .expect("query availability");

        assert_eq!(result.start_ms, 100_000);
        assert_eq!(result.end_ms, 200_000);
        assert_eq!(result.buckets.len(), 1);
        let bucket = &result.buckets[0];
        assert_eq!(bucket.provider_id, 42);
        assert_eq!(bucket.provider_name, "New Name");
        assert_eq!(bucket.requests_total, 2);
        assert_eq!(
            bucket.requests_success, 1,
            "3xx remains available even with an error marker"
        );
        assert_eq!(bucket.total_duration_ms, 400.0);
    }

    #[test]
    fn availability_assigns_each_event_to_its_own_time_bucket() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("availability-buckets.db"))
            .expect("init db");
        let conn = db.open_connection().expect("open db");
        conn.execute(
            "UPDATE usage_ledger_backfill_state SET status = 'complete' WHERE id = 1",
            [],
        )
        .expect("complete backfill");

        insert_ledger_event(&conn, 1, "Provider", Some(200), 0, 100, 100_000, 0);
        insert_ledger_event(&conn, 2, "Provider", Some(500), 0, 200, 350_000, 0);

        let result = availability_timeline_v1_with_conn(
            &conn,
            &UsageAvailabilityParams {
                lookback_ms: None,
                start_ms: Some(0),
                end_ms: Some(600_000),
                cli_key: Some("codex".to_string()),
                provider_id: Some(42),
            },
            999_999,
        )
        .expect("query availability");

        assert_eq!(result.bucket_size_ms, 5 * MINUTE_MS);
        assert_eq!(result.buckets.len(), 2);
        assert_eq!(result.buckets[0].bucket_start_ms, 0);
        assert_eq!(result.buckets[0].requests_total, 1);
        assert_eq!(result.buckets[0].requests_success, 1);
        assert_eq!(result.buckets[0].total_duration_ms, 100.0);
        assert_eq!(result.buckets[1].bucket_start_ms, 5 * MINUTE_MS);
        assert_eq!(result.buckets[1].requests_total, 1);
        assert_eq!(result.buckets[1].requests_success, 0);
        assert_eq!(result.buckets[1].total_duration_ms, 200.0);
    }

    #[test]
    fn availability_validates_bounds_and_filters() {
        let invalid = UsageAvailabilityParams {
            lookback_ms: Some(1),
            start_ms: Some(1),
            end_ms: Some(2),
            cli_key: None,
            provider_id: None,
        };
        assert!(resolve_bounds(&invalid, 100).is_err());

        let rolling = UsageAvailabilityParams {
            lookback_ms: Some(DAY_MS),
            start_ms: None,
            end_ms: None,
            cli_key: None,
            provider_id: None,
        };
        assert_eq!(
            resolve_bounds(&rolling, 200_000_000).expect("resolve rolling bounds"),
            (113_600_000, 200_000_000)
        );
    }

    #[test]
    fn availability_complete_mode_uses_the_ledger_time_index_for_coarse_range_filtering() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db =
            crate::db::init_for_tests(&dir.path().join("availability-plan.db")).expect("init db");
        let conn = db.open_connection().expect("open db");
        conn.execute(
            "UPDATE usage_ledger_backfill_state SET status = 'complete' WHERE id = 1",
            [],
        )
        .expect("complete backfill");

        let mut stmt = conn
            .prepare(
                "EXPLAIN QUERY PLAN SELECT * FROM usage_events WHERE created_at >= ?1 AND created_at <= ?2",
            )
            .expect("prepare query plan");
        let rows = stmt
            .query_map(params![100i64, 200i64], |row| row.get::<_, String>(3))
            .expect("query plan");
        let details = rows
            .collect::<Result<Vec<_>, _>>()
            .expect("read query plan")
            .join("\n");
        assert!(
            details.contains("idx_usage_ledger_created_at"),
            "complete ledger branch should use its standalone time index: {details}"
        );
    }
}
