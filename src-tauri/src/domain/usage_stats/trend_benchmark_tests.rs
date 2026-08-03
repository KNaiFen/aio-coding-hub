use super::cache_rate_trend_v1::{
    provider_cache_rate_trend_v1_with_conn, ProviderCacheRateTrendQuery,
};
use super::metrics_trend_v1::{
    provider_metric_trend_v1_with_conn, ProviderMetricTrendQuery,
};
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

    let metric_query = ProviderMetricTrendQuery {
        start_ts: Some(start_ts),
        end_ts: Some(start_ts + 31_536_000),
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

    provider_metric_trend_v1_with_conn(&conn, metric_query).expect("warm metric trend query");
    provider_cache_rate_trend_v1_with_conn(&conn, cache_query).expect("warm cache trend query");

    let metric_started = std::time::Instant::now();
    let metric_rows =
        provider_metric_trend_v1_with_conn(&conn, metric_query).expect("benchmark metric trend");
    let metric_elapsed = metric_started.elapsed();

    let cache_started = std::time::Instant::now();
    let cache_rows =
        provider_cache_rate_trend_v1_with_conn(&conn, cache_query).expect("benchmark cache trend");
    let cache_elapsed = cache_started.elapsed();

    assert!(!metric_rows.is_empty());
    assert!(!cache_rows.is_empty());
    let budget = std::time::Duration::from_secs(1);
    assert!(
        metric_elapsed < budget,
        "million-row metric trend query took {metric_elapsed:?}, budget is {budget:?}"
    );
    assert!(
        cache_elapsed < budget,
        "million-row cache trend query took {cache_elapsed:?}, budget is {budget:?}"
    );
}
