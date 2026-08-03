use super::cache_rate_trend_v1::{
    provider_cache_rate_trend_v1_with_conn, ProviderCacheRateTrendQuery,
};
use super::metrics_trend_v1::{provider_metric_trend_v1_with_conn, ProviderMetricTrendQuery};
use crate::db;
use rusqlite::params;

#[test]
#[ignore = "release-only million-row provider trend performance gate"]
fn provider_trend_million_ledger_rows_release_under_one_second() {
    assert!(
        !cfg!(debug_assertions),
        "provider trend performance gate must run with --release"
    );

    let temp_dir = tempfile::tempdir().expect("create benchmark temp directory");
    let database_path = temp_dir.path().join("provider-trend-benchmark.db");
    let db = db::init_for_tests(&database_path).expect("initialize benchmark database");
    let conn = db.open_connection().expect("open benchmark database");
    let start_ts = 1_704_067_200i64;

    conn.execute(
        r#"
WITH digits(d) AS (
  VALUES (0), (1), (2), (3), (4), (5), (6), (7), (8), (9)
),
million(n) AS (
  SELECT (((((d0.d * 10 + d1.d) * 10 + d2.d) * 10 + d3.d) * 10 + d4.d) * 10 + d5.d)
  FROM digits d0
  CROSS JOIN digits d1
  CROSS JOIN digits d2
  CROSS JOIN digits d3
  CROSS JOIN digits d4
  CROSS JOIN digits d5
)
INSERT INTO usage_ledger (
  request_log_id,
  trace_id,
  cli_key,
  created_at,
  created_at_ms,
  status,
  error_present,
  excluded_from_stats,
  duration_ms,
  ttfb_ms,
  final_provider_id,
  provider_name_snapshot,
  usage_present,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens
)
SELECT
  n + 1,
  'provider-trend-benchmark-' || (n + 1),
  CASE n % 4
    WHEN 0 THEN 'codex'
    WHEN 1 THEN 'claude'
    WHEN 2 THEN 'gemini'
    ELSE 'grok'
  END,
  ?1 + ((n * 31) % 31536000),
  (?1 + ((n * 31) % 31536000)) * 1000,
  200,
  0,
  0,
  800 + (n % 401),
  100 + (n % 101),
  (n % 10) + 1,
  printf('Provider %d', (n % 10) + 1),
  1,
  100 + (n % 1000),
  20 + (n % 200),
  120 + (n % 1200),
  n % 50,
  n % 20
FROM million
        "#,
        params![start_ts],
    )
    .expect("insert one million ledger rows");
    conn.execute(
        r#"
UPDATE usage_ledger_backfill_state
SET status = 'complete',
    target_request_log_id = 1000000,
    last_request_log_id = 1000000,
    completed_at = ?1,
    updated_at = ?1
WHERE id = 1
        "#,
        params![start_ts],
    )
    .expect("activate ledger-only usage view");

    drop(conn);
    let refresh_started = std::time::Instant::now();
    let refresh_report = crate::usage_provider_daily_rollup::refresh_completed_days(
        &db,
        start_ts + 31_536_000 + 2 * 86_400,
    )
    .expect("backfill Provider daily rollups before timing queries");
    let refresh_elapsed = refresh_started.elapsed();
    assert!(refresh_report.ledger_backfill_complete);
    assert!(
        refresh_report.rebuilt_days >= 300,
        "million-row gate did not exercise the daily rollup backfill: {refresh_report:?}"
    );
    eprintln!(
        "Provider daily rollup backfill completed in {refresh_elapsed:?}; excluded from the one-second query budget"
    );

    let conn = db.open_connection().expect("reopen benchmark database");
    let (query_start_ts, query_end_ts, covered_seconds): (i64, i64, i64) = conn
        .query_row(
            r#"
SELECT
  MIN(day_start_ts),
  MAX(day_end_ts),
  SUM(day_end_ts - day_start_ts)
FROM usage_provider_daily_rollup_days
WHERE status = 'complete'
"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("resolve fully covered Provider rollup range");
    assert_eq!(
        covered_seconds,
        query_end_ts - query_start_ts,
        "benchmark range must be fully and contiguously covered by complete daily rollups"
    );
    let (rollup_groups, projected_requests): (i64, i64) = conn
        .query_row(
            r#"
SELECT COUNT(*), COALESCE(SUM(requests_total), 0)
FROM usage_provider_daily_rollups
"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("inspect compressed Provider rollups");
    assert_eq!(projected_requests, 1_000_000);
    assert!(
        rollup_groups < 50_000,
        "million raw rows were not materially compressed: {rollup_groups} rollup groups"
    );

    let metric_query = ProviderMetricTrendQuery {
        start_ts: Some(query_start_ts),
        end_ts: Some(query_end_ts),
        cli_key: None,
        provider_id: None,
        limit: None,
        exclude_cx2cc_gateway_bridge: false,
    };
    let cache_query = ProviderCacheRateTrendQuery {
        start_ts: metric_query.start_ts,
        end_ts: metric_query.end_ts,
        cli_key: None,
        provider_id: None,
        limit: None,
        exclude_cx2cc_gateway_bridge: false,
    };
    let all_time_metric_query = ProviderMetricTrendQuery {
        start_ts: None,
        end_ts: None,
        ..metric_query
    };
    let all_time_cache_query = ProviderCacheRateTrendQuery {
        start_ts: None,
        end_ts: None,
        ..cache_query
    };

    provider_metric_trend_v1_with_conn(&conn, metric_query).expect("warm metric trend query");
    provider_cache_rate_trend_v1_with_conn(&conn, cache_query).expect("warm cache trend query");
    provider_metric_trend_v1_with_conn(&conn, all_time_metric_query)
        .expect("warm all-time metric trend query");
    provider_cache_rate_trend_v1_with_conn(&conn, all_time_cache_query)
        .expect("warm all-time cache trend query");

    let metric_started = std::time::Instant::now();
    let metric_rows =
        provider_metric_trend_v1_with_conn(&conn, metric_query).expect("benchmark metric trend");
    let metric_elapsed = metric_started.elapsed();

    let cache_started = std::time::Instant::now();
    let cache_rows =
        provider_cache_rate_trend_v1_with_conn(&conn, cache_query).expect("benchmark cache trend");
    let cache_elapsed = cache_started.elapsed();

    let all_time_metric_started = std::time::Instant::now();
    let all_time_metric_rows =
        provider_metric_trend_v1_with_conn(&conn, all_time_metric_query)
            .expect("benchmark all-time metric trend");
    let all_time_metric_elapsed = all_time_metric_started.elapsed();

    let all_time_cache_started = std::time::Instant::now();
    let all_time_cache_rows = provider_cache_rate_trend_v1_with_conn(&conn, all_time_cache_query)
        .expect("benchmark all-time cache trend");
    let all_time_cache_elapsed = all_time_cache_started.elapsed();

    assert!(!metric_rows.is_empty());
    assert!(!cache_rows.is_empty());
    assert!(!all_time_metric_rows.is_empty());
    assert!(!all_time_cache_rows.is_empty());
    let budget = std::time::Duration::from_secs(1);
    assert!(
        metric_elapsed < budget,
        "million-row metric trend query took {metric_elapsed:?}, budget is {budget:?}"
    );
    assert!(
        cache_elapsed < budget,
        "million-row cache trend query took {cache_elapsed:?}, budget is {budget:?}"
    );
    assert!(
        all_time_metric_elapsed < budget,
        "million-row all-time metric trend query took {all_time_metric_elapsed:?}, budget is {budget:?}"
    );
    assert!(
        all_time_cache_elapsed < budget,
        "million-row all-time cache trend query took {all_time_cache_elapsed:?}, budget is {budget:?}"
    );
}
